#![allow(dead_code, unused_imports)]

use std::collections::HashSet;

use chrono::{Duration, Utc};
use warp_core::settings::Setting;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::ai::blocklist::view_util::{format_dollars, usage_display_unit};
use crate::ai::request_usage_model::{
    AIRequestUsageModel, AIRequestUsageModelEvent, BonusGrant, BonusGrantScope,
};
use crate::settings::UsageDisplayUnit;
use crate::terminal::general_settings::GeneralSettings;

pub struct BonusGrantNotificationModel {
    /// In-memory tracking of grants shown during this session. This prevents duplicate
    /// notifications when multiple `AIRequestUsageModelEvent::RequestUsageUpdated` events
    /// fire in quick succession before the persisted settings can be updated.
    shown_grants_session: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum BonusGrantNotificationEvent {
    ShowNotification { grant: BonusGrant, message: String },
}

impl Entity for BonusGrantNotificationModel {
    type Event = BonusGrantNotificationEvent;
}

impl SingletonEntity for BonusGrantNotificationModel {}

impl BonusGrantNotificationModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&AIRequestUsageModel::handle(ctx), |me, _, event, ctx| {
            if let AIRequestUsageModelEvent::RequestUsageUpdated = event {
                me.check_for_new_bonus_grants(ctx);
            }
        });

        Self {
            shown_grants_session: HashSet::new(),
        }
    }

    fn check_for_new_bonus_grants(&mut self, ctx: &mut ModelContext<Self>) {
        let _ = ctx;
    }

    /// Describes a grant in dollars when displaying in dollars and the grant carries a dollar
    /// value, otherwise in credits.
    fn format_generic_grant_message(grant: &BonusGrant, unit: UsageDisplayUnit) -> String {
        let scope_text = match grant.scope {
            BonusGrantScope::User => "account",
            BonusGrantScope::Team(_) => "team",
            BonusGrantScope::Workspace(_) => "workspace",
        };
        let usage_cents_granted = match unit {
            UsageDisplayUnit::Dollars => grant.usage_cents_granted,
            UsageDisplayUnit::Credits => None,
        };
        match usage_cents_granted {
            Some(cents) => format!(
                "{} has been added to your {scope_text}.",
                format_dollars(cents as f32)
            ),
            None => format!(
                "{} Reload Credits have been added to your {scope_text}.",
                grant.request_credits_granted
            ),
        }
    }

    fn create_grant_key(grant: &BonusGrant) -> String {
        format!("{}:{}", grant.reason, grant.created_at.timestamp())
    }

    fn mark_grant_as_shown(&self, grant_key: &str, ctx: &mut ModelContext<Self>) {
        GeneralSettings::handle(ctx).update(ctx, |settings, ctx| {
            let mut shown_grants = settings.bonus_grants_shown.value().clone();
            shown_grants.insert(grant_key.to_string());

            if let Err(e) = settings.bonus_grants_shown.set_value(shown_grants, ctx) {
                log::warn!("Failed to mark bonus grant as shown: {e}");
            }
        });
    }
}

#[cfg(test)]
#[path = "bonus_grant_notification_model_tests.rs"]
mod tests;
