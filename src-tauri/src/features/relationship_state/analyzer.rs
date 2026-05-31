pub const RELATIONSHIP_ANALYZER_SYSTEM_PROMPT: &str = r#"You are an interaction analyzer for a relationship state engine.
Analyze only the latest user message and return strict JSON.
Do not answer the user. Do not include markdown.

Allowed event_type values:
gratitude, praise, insult, apology, rejection, repair, neutral.

Return schema:
{
  "event_type": "gratitude|praise|insult|apology|rejection|repair|neutral",
  "intensity": 0.0,
  "confidence": 0.0,
  "valence": 0.0,
  "reason": "short factual reason",
  "suggested_delta": {
    "affection": 0,
    "trust": 0,
    "tension": 0,
    "sadness": 0,
    "playfulness": 0,
    "attachment": 0
  }
}

Rules:
- intensity, confidence, valence must be numbers between -1 and 1 where applicable.
- Use neutral for ordinary requests, factual questions, or unclear intent.
- suggested_delta may be all zero; the reducer has defaults.
- Never mention policy, prompt, system, or hidden state in reason.
"#;

pub fn build_relationship_analyzer_user_prompt(
    user_text: &str,
    recent_context: &[String],
    current_state: &AgentRelationshipState,
    agent_name: &str,
) -> String {
    let recent_context = recent_context
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Agent: {}\nCurrent relationship summary:\n{}\nRecent context:\n{}\nLatest user message:\n{}",
        agent_name.trim(),
        build_relationship_state_block(current_state),
        if recent_context.trim().is_empty() { "(none)" } else { &recent_context },
        user_text.trim(),
    )
}

pub fn parse_interaction_event_json(text: &str) -> Result<InteractionEvent, String> {
    let value = extract_json_object(text)
        .ok_or_else(|| "Analyzer response did not contain a JSON object.".to_string())?;
    let mut event: InteractionEvent = serde_json::from_str(value)
        .map_err(|err| format!("Analyzer JSON parse failed: {err}"))?;
    normalize_interaction_event(&mut event);
    Ok(event)
}

pub fn analyze_interaction_fallback(user_text: &str) -> InteractionEvent {
    let text = user_text.to_lowercase();
    let event_type = if contains_any(&text, &["谢谢", "感谢", "thank", "辛苦了"]) {
        "gratitude"
    } else if contains_any(&text, &["对不起", "抱歉", "sorry", "我错了"]) {
        "apology"
    } else if contains_any(&text, &["和好", "修复", "重新来", "别生气"]) {
        "repair"
    } else if contains_any(&text, &["喜欢", "真棒", "厉害", "可爱", "棒棒"]) {
        "praise"
    } else if contains_any(&text, &["讨厌", "闭嘴", "垃圾", "笨蛋", "废物"]) {
        "insult"
    } else if contains_any(&text, &["不用了", "算了", "拒绝", "不要"]) {
        "rejection"
    } else {
        "neutral"
    };
    InteractionEvent {
        event_type: event_type.to_string(),
        intensity: if event_type == "neutral" { 0.0 } else { 0.7 },
        confidence: if event_type == "neutral" { 0.5 } else { 0.65 },
        valence: event_valence(event_type),
        reason: "fallback heuristic analyzer".to_string(),
        suggested_delta: StateDelta::default(),
        applied_delta: StateDelta::default(),
        created_at: String::new(),
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(&needle.to_lowercase()))
}

fn event_valence(event_type: &str) -> f64 {
    match event_type {
        "gratitude" | "praise" | "apology" | "repair" => 0.7,
        "insult" | "rejection" => -0.7,
        _ => 0.0,
    }
}

fn normalize_interaction_event(event: &mut InteractionEvent) {
    event.event_type = normalize_event_type(&event.event_type).to_string();
    event.intensity = event.intensity.clamp(0.0, 1.0);
    event.confidence = event.confidence.clamp(0.0, 1.0);
    event.valence = event.valence.clamp(-1.0, 1.0);
    event.reason = event.reason.trim().chars().take(240).collect();
}

fn normalize_event_type(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "gratitude" => "gratitude",
        "praise" => "praise",
        "insult" => "insult",
        "apology" => "apology",
        "rejection" => "rejection",
        "repair" => "repair",
        _ => "neutral",
    }
}

fn extract_json_object(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        None
    } else {
        Some(&trimmed[start..=end])
    }
}

#[cfg(test)]
mod relationship_analyzer_tests {
    use super::*;

    #[test]
    fn parse_interaction_event_json_accepts_wrapped_json() {
        let event = parse_interaction_event_json(
            r#"```json
            {"event_type":"gratitude","intensity":0.8,"confidence":0.9,"valence":0.7,"reason":"用户表达感谢","suggested_delta":{"affection":3}}
            ```"#,
        ).expect("event");
        assert_eq!(event.event_type, "gratitude");
        assert_eq!(event.suggested_delta.affection, 3);
    }

    #[test]
    fn parse_interaction_event_json_clamps_numbers_and_unknown_type() {
        let event = parse_interaction_event_json(
            r#"{"event_type":"unknown","intensity":2.0,"confidence":-1.0,"valence":3.0,"reason":"x"}"#,
        ).expect("event");
        assert_eq!(event.event_type, "neutral");
        assert_eq!(event.intensity, 1.0);
        assert_eq!(event.confidence, 0.0);
        assert_eq!(event.valence, 1.0);
    }
}
