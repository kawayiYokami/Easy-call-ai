fn frontend_tool_definition(function: ProviderToolDefinition) -> FrontendToolDefinition {
    FrontendToolDefinition {
        kind: "function".to_string(),
        function: FrontendToolFunctionDefinition {
            name: function.name,
            description: function.description,
            parameters: function.parameters,
        },
    }
}

async fn builtin_tool_definitions_for_frontend(
    state: &AppState,
) -> Vec<FrontendToolDefinition> {
    let preview_session_id = "__frontend_tool_preview__".to_string();
    let _preview_api_id = "__frontend_tool_preview__".to_string();
    let preview_agent_id = DEFAULT_AGENT_ID.to_string();
    let preview_memory_context = build_memory_agent_context(&preview_agent_id, false)
        .unwrap_or(MemoryAgentContext {
            owner_agent_id: None,
            effective_agent_id: preview_agent_id.clone(),
            private_memory_enabled: false,
        });
    let out = vec![
        frontend_tool_definition(
            BuiltinFetchTool {
                app_state: state.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinBingSearchTool {
                app_state: state.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinRememberTool {
                app_state: state.clone(),
                memory_context: preview_memory_context.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinRecallTool {
                app_state: state.clone(),
                memory_context: preview_memory_context.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(operate_provider_tool_definition()),
        frontend_tool_definition(
            BuiltinReloadTool {
                app_state: state.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(read_provider_tool_definition()),
        frontend_tool_definition(
            BuiltinPlanTool {
                app_state: state.clone(),
                session_id: preview_session_id.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinTerminalExecTool {
                app_state: state.clone(),
                session_id: preview_session_id.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinApplyPatchTool {
                app_state: state.clone(),
                session_id: preview_session_id.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinTodoTool {
                app_state: state.clone(),
                session_id: preview_session_id.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinTaskTool {
                app_state: state.clone(),
                session_id: preview_session_id.clone(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinDelegateTool {
                app_state: state.clone(),
                session_id: preview_session_id,
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinContactReplyTool {
                app_state: state.clone(),
                session_id: "__frontend_tool_preview__".to_string(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(
            BuiltinContactSendFilesTool {
                app_state: state.clone(),
                session_id: "__frontend_tool_preview__".to_string(),
            }
            .provider_tool_definition(),
        ),
        frontend_tool_definition(BuiltinContactNoReplyTool.provider_tool_definition()),
        frontend_tool_definition(
            BuiltinMemeTool {
                app_state: state.clone(),
            }
            .provider_tool_definition(),
        ),
    ];
    out
}

fn department_permission_catalog_item(name: &str, description: &str) -> Option<DepartmentPermissionCatalogItem> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(DepartmentPermissionCatalogItem {
        name: name.to_string(),
        description: description.trim().to_string(),
    })
}

fn sorted_unique_catalog_items(
    values: impl IntoIterator<Item = DepartmentPermissionCatalogItem>,
) -> Vec<DepartmentPermissionCatalogItem> {
    let mut out = values.into_iter().collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

#[tauri::command]
async fn list_tool_catalog(state: State<'_, AppState>) -> Result<Vec<FrontendToolDefinition>, String> {
    Ok(builtin_tool_definitions_for_frontend(state.inner()).await)
}

#[tauri::command]
async fn list_department_permission_catalog(
    state: State<'_, AppState>,
) -> Result<DepartmentPermissionCatalog, String> {
    let builtin_tools = sorted_unique_catalog_items(
        builtin_tool_definitions_for_frontend(state.inner())
            .await
            .into_iter()
            .filter_map(|item| {
                if !builtin_tool_visible_in_department_permissions(&item.function.name) {
                    return None;
                }
                department_permission_catalog_item(
                    &item.function.name,
                    &item.function.description,
                )
            }),
    );

    let skills = load_workspace_skill_summaries_with_errors(&state)
        .map(|(skills, _errors)| {
            sorted_unique_catalog_items(skills.into_iter().filter_map(|item| {
                department_permission_catalog_item(&item.name, &item.description)
            }))
        })
        .unwrap_or_default();
    let mcp_tools = sorted_unique_catalog_items(
        load_workspace_mcp_servers(&state)?
            .into_iter()
            .flat_map(|server| {
                let server_name = server.name.clone();
                list_tools_from_runtime_or_policy(&server)
                    .into_iter()
                    .filter_map(move |tool| {
                        department_permission_catalog_item(
                            &format!("{}::{}", server_name, tool.tool_name),
                            &tool.description,
                        )
                    })
            }),
    );
    Ok(DepartmentPermissionCatalog {
        builtin_tools,
        skills,
        mcp_tools,
    })
}

#[cfg(test)]
mod tool_catalog_tests {
    use super::*;

    fn frontend_definition_json(definition: &FrontendToolDefinition) -> serde_json::Value {
        serde_json::to_value(definition).expect("serialize frontend tool definition should succeed")
    }

    async fn catalog_tool_definition_by_name(tool_name: &str) -> Result<FrontendToolDefinition, String> {
        let state = AppState::new()?;
        builtin_tool_definitions_for_frontend(&state)
            .await
            .into_iter()
            .find(|definition| definition.function.name == tool_name)
            .ok_or_else(|| format!("frontend catalog tool definition not found: {tool_name}"))
    }

    #[test]
    fn frontend_catalog_tools_should_match_runtime_definitions() {
        let operate_catalog = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime for tool catalog tests should succeed")
            .block_on(catalog_tool_definition_by_name(MCP_OPERATE_TOOL_NAME))
            .expect("load operate definition from frontend catalog should succeed");
        let operate_runtime =
            frontend_tool_definition(operate_provider_tool_definition());
        assert_eq!(
            frontend_definition_json(&operate_catalog),
            frontend_definition_json(&operate_runtime),
            "frontend catalog operate definition drifted from builtin definition"
        );

        let read_catalog = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime for tool catalog tests should succeed")
            .block_on(catalog_tool_definition_by_name(READ_TOOL_NAME))
            .expect("load read definition from frontend catalog should succeed");
        let read_runtime =
            frontend_tool_definition(read_provider_tool_definition());
        assert_eq!(
            frontend_definition_json(&read_catalog),
            frontend_definition_json(&read_runtime),
            "frontend catalog read definition drifted from builtin definition"
        );

        let todo_catalog = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime for tool catalog tests should succeed")
            .block_on(catalog_tool_definition_by_name(TODO_TOOL_NAME))
            .expect("load todo definition from frontend catalog should succeed");
        let todo_runtime = frontend_tool_definition(
            BuiltinTodoTool {
                app_state: AppState::new().expect("create app state for todo definition"),
                session_id: "__frontend_tool_preview__".to_string(),
            }
            .provider_tool_definition(),
        );
        assert_eq!(
            frontend_definition_json(&todo_catalog),
            frontend_definition_json(&todo_runtime),
            "frontend catalog todo definition drifted from runtime builtin definition"
        );
    }
}
