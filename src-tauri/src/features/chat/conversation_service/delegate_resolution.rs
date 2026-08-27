impl ConversationServiceV2 {
    fn resolve_delegate_context(
        &self,
        app_state: &AppState,
        source_agent_id: &str,
        source_department_id: Option<&str>,
        source_conversation_id: Option<&str>,
        target_department_id: &str,
        target_agent_id: Option<&str>,
    ) -> Result<DelegateContextResolution, String> {
        let guard = app_state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime_snapshot = load_runtime_organization_snapshot(app_state)?;
        let requested_source_conversation_id = source_conversation_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let thread_context = if let Some(conversation_id) = requested_source_conversation_id {
            delegate_runtime_thread_get(app_state, conversation_id)?
        } else {
            None
        };
        let source_conversation = if let Some(thread) = thread_context.as_ref() {
            Some(thread.conversation.clone())
        } else if let Some(conversation_id) = requested_source_conversation_id {
            Some(
                self.get_conversation_meta(app_state, conversation_id)
                    .ok()
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.conversation_kind.trim()
                                != CONVERSATION_KIND_DELEGATE
                    })
                    .map(|conversation_meta| {
                        self.build_conversation_record_from_meta_view(&conversation_meta)
                    })
                    .ok_or_else(|| {
                        format!("未找到指定来源会话，conversationId={conversation_id}")
                    })?,
            )
        } else {
            None
        };
        let requested_source_department_id = source_department_id
            .map(str::trim)
            .filter(|department_id| !department_id.is_empty());
        let source_department = if let Some(department_id) = requested_source_department_id {
            runtime_department_by_id(&runtime_snapshot, department_id)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "未找到发起部门，departmentId={}，agentId={}",
                        department_id, source_agent_id
                    )
                })?
        } else {
            source_conversation
                .as_ref()
                .and_then(|conversation| {
                    let department_id = conversation.department_id.trim();
                    if department_id.is_empty() {
                        None
                    } else {
                        runtime_department_by_id(&runtime_snapshot, department_id).cloned()
                    }
                })
                .ok_or_else(|| format!("未找到发起部门，agentId={source_agent_id}"))?
        };
        let target_department = runtime_department_by_id(&runtime_snapshot, target_department_id)
            .cloned()
            .or_else(|| {
                runtime_snapshot
                    .config
                    .departments
                    .iter()
                    .find(|department| {
                        department.name.trim().eq_ignore_ascii_case(target_department_id.trim())
                    })
                    .cloned()
            })
            .ok_or_else(|| {
                format!("目标部门不存在，departmentId={target_department_id}")
            })?;
        let target_agent_id = if let Some(requested_agent_id) = target_agent_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !target_department
                .agent_ids
                .iter()
                .any(|id| id.trim() == requested_agent_id)
            {
                drop(guard);
                return Err(format!(
                    "目标委任人不属于目标部门，departmentId={}，agentId={}",
                    target_department_id, requested_agent_id
                ));
            }
            if available_non_user_agent(&runtime_snapshot.agents, requested_agent_id).is_none() {
                drop(guard);
                return Err(format!("目标委任人不存在，agentId={requested_agent_id}"));
            }
            requested_agent_id.to_string()
        } else if let Some(agent) =
            first_available_department_agent(&target_department, &runtime_snapshot.agents)
        {
            agent.id.clone()
        } else {
            available_non_user_agent(&runtime_snapshot.agents, DEPUTY_AGENT_ID)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| {
                    format!(
                        "目标部门没有可用委任人，且副手人格不可用，departmentId={target_department_id}"
                    )
                })?
        };
        let source_conversation_id = if let Some(thread) = thread_context.as_ref() {
            thread.root_conversation_id.clone()
        } else {
            source_conversation
                .as_ref()
                .map(|conversation| conversation.id.clone())
                .ok_or_else(|| "主代理缺少当前会话 ID，无法发起委托".to_string())?
        };
        drop(guard);
        Ok(DelegateContextResolution {
            config: runtime_snapshot.config,
            agents: runtime_snapshot.agents,
            source_department,
            target_department,
            target_agent_id,
            source_conversation_id,
            thread_context,
        })
    }

    fn resolve_delegate_result_target_conversation(
        &self,
        state: &AppState,
        root_conversation_id: &str,
    ) -> Result<DelegateResultTargetConversationResolution, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let assistant_agent_id = state_service_get_assistant_department_agent_id(state)?;
        let department_id = runtime_department_for_agent(&runtime_snapshot, &assistant_agent_id)
            .map(|item| item.id.clone())
            .unwrap_or_else(|| ASSISTANT_DEPARTMENT_ID.to_string());
        let normalized_root_conversation_id = root_conversation_id.trim();
        let mut conversation_to_persist = None::<Conversation>;
        let target_conversation_id =
            if task_conversation_id_is_system_notification(normalized_root_conversation_id) {
                if let Some(conversation_meta) = self
                    .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
                    .ok()
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                            && self.conversation_meta_is_system_notification_meta_view(
                                conversation_meta,
                            )
                    })
                {
                    conversation_meta.id
                } else {
                    let conversation = build_system_notification_conversation_record();
                    let conversation_id = conversation.id.clone();
                    conversation_to_persist = Some(conversation);
                    conversation_id
                }
            } else if self
                .get_conversation_meta(state, normalized_root_conversation_id)
                .ok()
                .filter(|conversation_meta| {
                    self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                        && conversation_meta.conversation_kind.trim()
                            != CONVERSATION_KIND_DELEGATE
                        && conversation_meta.conversation_kind.trim()
                            != CONVERSATION_KIND_SYSTEM_NOTIFICATION
                })
                .is_some()
            {
                normalized_root_conversation_id.to_string()
            } else {
                return Err(format!(
                    "委托绑定会话不存在，无法写回结果，conversationId={normalized_root_conversation_id}"
                ));
            };
        drop(guard);
        if let Some(conversation) = conversation_to_persist {
            state_schedule_conversation_persist(state, &conversation)?;
        }
        Ok(DelegateResultTargetConversationResolution {
            department_id,
            agent_id: assistant_agent_id,
            target_conversation_id,
        })
    }

}

