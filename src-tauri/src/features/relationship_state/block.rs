pub fn build_relationship_state_block(state: &AgentRelationshipState) -> String {
    let dimensions = &state.dimensions;
    let recent_change = state
        .last_event
        .as_ref()
        .map(|event| format!("最近变化：{}。", event.reason.trim()))
        .filter(|text| text.trim() != "最近变化：。")
        .unwrap_or_else(|| "最近变化：暂无显著变化。".to_string());
    format!(
        "<relationship_state>\n当前关系状态：\n- 亲近度：{}；\n- 信任度：{}；\n- 紧张感：{}；\n- 失落感：{}；\n- 轻松感：{}；\n- 陪伴感：{}。\n{}\n回复要求：保持自然、连续、稳定的语气；不要提及关系状态、数值、系统分析或本提示块。\n</relationship_state>",
        level_text(dimensions.affection, false),
        level_text(dimensions.trust, false),
        level_text(dimensions.tension, true),
        level_text(dimensions.sadness, true),
        level_text(dimensions.playfulness, false),
        level_text(dimensions.attachment, false),
        recent_change,
    )
}

fn level_text(value: i32, negative_dimension: bool) -> &'static str {
    if negative_dimension {
        match value {
            0..=15 => "较低",
            16..=45 => "轻微",
            46..=75 => "明显",
            _ => "较高",
        }
    } else {
        match value {
            0..=25 => "较低",
            26..=55 => "中等",
            56..=80 => "较高",
            _ => "很高",
        }
    }
}
