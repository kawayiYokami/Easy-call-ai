pub fn reduce_relationship_state(
    state: &mut AgentRelationshipState,
    mut event: InteractionEvent,
    rules: &RelationshipRules,
) -> InteractionEvent {
    apply_decay(&mut state.dimensions, rules);

    if event.suggested_delta == StateDelta::default() {
        if let Some(default_delta) = rules.event_impacts.get(event.event_type.as_str()) {
            event.suggested_delta = default_delta.clone();
        }
    }

    let applied = scaled_delta(&state.dimensions, &event.suggested_delta, event.intensity, event.confidence);
    apply_delta_to_dimensions(&mut state.dimensions, &applied, rules);

    event.applied_delta = applied;
    if event.created_at.trim().is_empty() {
        event.created_at = chrono_now();
    }
    state.updated_at = event.created_at.clone();
    state.turn_count += 1;
    state.last_event = Some(event.clone());
    state.recent_events.push(event.clone());
    if state.recent_events.len() > rules.recent_event_limit {
        let overflow = state.recent_events.len() - rules.recent_event_limit;
        state.recent_events.drain(0..overflow);
    }
    event
}

fn apply_decay(dimensions: &mut RelationshipDimensions, rules: &RelationshipRules) {
    let decay = &rules.decay_per_turn;
    dimensions.affection += decay.affection;
    dimensions.trust += decay.trust;
    dimensions.tension += decay.tension;
    dimensions.sadness += decay.sadness;
    dimensions.playfulness += decay.playfulness;
    dimensions.attachment += decay.attachment;
    clamp_dimensions(dimensions, rules);
}

fn scaled_delta(
    dimensions: &RelationshipDimensions,
    delta: &StateDelta,
    intensity: f64,
    confidence: f64,
) -> StateDelta {
    let factor = intensity.clamp(0.0, 1.0) * confidence.clamp(0.0, 1.0);
    StateDelta {
        affection: scale_one(delta.affection, dimensions.affection, factor),
        trust: scale_one(delta.trust, dimensions.trust, factor),
        tension: scale_one(delta.tension, dimensions.tension, factor),
        sadness: scale_one(delta.sadness, dimensions.sadness, factor),
        playfulness: scale_one(delta.playfulness, dimensions.playfulness, factor),
        attachment: scale_one(delta.attachment, dimensions.attachment, factor),
    }
}

fn scale_one(delta: i32, current_value: i32, factor: f64) -> i32 {
    let damping = if delta > 0 && current_value >= 90 {
        0.35
    } else if delta > 0 && current_value >= 75 {
        0.65
    } else if delta < 0 && current_value <= 10 {
        0.35
    } else if delta < 0 && current_value <= 25 {
        0.65
    } else {
        1.0
    };
    ((delta as f64) * factor * damping).round() as i32
}

fn apply_delta_to_dimensions(
    dimensions: &mut RelationshipDimensions,
    delta: &StateDelta,
    rules: &RelationshipRules,
) {
    dimensions.affection += delta.affection;
    dimensions.trust += delta.trust;
    dimensions.tension += delta.tension;
    dimensions.sadness += delta.sadness;
    dimensions.playfulness += delta.playfulness;
    dimensions.attachment += delta.attachment;
    clamp_dimensions(dimensions, rules);
}

fn clamp_dimensions(dimensions: &mut RelationshipDimensions, rules: &RelationshipRules) {
    dimensions.affection = dimensions.affection.clamp(rules.floor.affection, rules.ceiling.affection);
    dimensions.trust = dimensions.trust.clamp(rules.floor.trust, rules.ceiling.trust);
    dimensions.tension = dimensions.tension.clamp(rules.floor.tension, rules.ceiling.tension);
    dimensions.sadness = dimensions.sadness.clamp(rules.floor.sadness, rules.ceiling.sadness);
    dimensions.playfulness = dimensions.playfulness.clamp(rules.floor.playfulness, rules.ceiling.playfulness);
    dimensions.attachment = dimensions.attachment.clamp(rules.floor.attachment, rules.ceiling.attachment);
}

impl PartialEq for StateDelta {
    fn eq(&self, other: &Self) -> bool {
        self.affection == other.affection
            && self.trust == other.trust
            && self.tension == other.tension
            && self.sadness == other.sadness
            && self.playfulness == other.playfulness
            && self.attachment == other.attachment
    }
}

impl Eq for StateDelta {}

#[cfg(test)]
mod relationship_state_reducer_tests {
    use super::*;

    fn event(event_type: &str, intensity: f64) -> InteractionEvent {
        InteractionEvent {
            event_type: event_type.to_string(),
            intensity,
            confidence: 1.0,
            valence: 0.0,
            reason: String::new(),
            suggested_delta: StateDelta::default(),
            applied_delta: StateDelta::default(),
            created_at: String::new(),
        }
    }

    #[test]
    fn stronger_event_applies_larger_delta() {
        let rules = RelationshipRules::default();
        let mut weak = AgentRelationshipState::default();
        let mut strong = AgentRelationshipState::default();
        reduce_relationship_state(&mut weak, event("gratitude", 0.25), &rules);
        reduce_relationship_state(&mut strong, event("gratitude", 1.0), &rules);
        assert!(strong.dimensions.affection > weak.dimensions.affection);
    }

    #[test]
    fn high_value_damping_reduces_positive_delta() {
        let rules = RelationshipRules::default();
        let mut state = AgentRelationshipState::default();
        state.dimensions.affection = 95;
        let applied = reduce_relationship_state(&mut state, event("gratitude", 1.0), &rules);
        assert!(applied.applied_delta.affection < 4);
    }

    #[test]
    fn apology_reduces_tension() {
        let rules = RelationshipRules::default();
        let mut state = AgentRelationshipState::default();
        state.dimensions.tension = 40;
        reduce_relationship_state(&mut state, event("apology", 1.0), &rules);
        assert!(state.dimensions.tension < 40);
    }
}
