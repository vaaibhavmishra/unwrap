//! Unwrap's loading symbol and status text.

use std::time::Duration;

use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::appearance::Appearance;
use warpui::elements::shimmering_text::{
    ShimmerConfig, ShimmeringTextElement, ShimmeringTextStateHandle,
};
use warpui::elements::{
    ConstrainedBox, CrossAxisAlignment, Element, Flex, Icon, ParentElement, Point, Shrinkable, Text,
};
use warpui::event::DispatchedEvent;
use warpui::{
    AfterLayoutContext, AppContext, EventContext, LayoutContext, PaintContext, SingletonEntity,
    SizeConstraint,
};

pub const LOADING_ICON_SPACING: f32 = 5.;

pub fn shimmering_warp_loading_text(
    text: impl Into<String>,
    font_size: f32,
    shimmer_handle: ShimmeringTextStateHandle,
    ctx: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(ctx);
    let theme = appearance.theme();
    let base_color = theme.disabled_text_color(theme.surface_1()).into_solid();
    let text_color = theme.main_text_color(theme.surface_1()).into_solid();
    #[cfg(target_os = "macos")]
    let reduced_motion =
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
    #[cfg(not(target_os = "macos"))]
    let reduced_motion = false;

    let text = text.into();
    let icon = LoadingIcon {
        icon: Icon::new("bundled/svg/unwrap-symbol.svg", text_color),
        clock: shimmer_handle.clone(),
        reduced_motion,
    };
    let text = if reduced_motion {
        Text::new_inline(text, appearance.ui_font_family(), font_size)
            .with_color(text_color)
            .with_selectable(false)
            .soft_wrap(false)
            .finish()
    } else {
        ShimmeringTextElement::new(
            text,
            appearance.ui_font_family(),
            font_size,
            base_color,
            text_color,
            ShimmerConfig::default(),
            shimmer_handle,
        )
        .finish()
    };

    Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(LOADING_ICON_SPACING)
        .with_child(
            ConstrainedBox::new(icon.finish())
                .with_width(font_size)
                .with_height(font_size)
                .finish(),
        )
        .with_child(Shrinkable::new(1., text).finish())
        .finish()
}

struct LoadingIcon {
    icon: Icon,
    clock: ShimmeringTextStateHandle,
    reduced_motion: bool,
}

impl Element for LoadingIcon {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        self.icon.layout(constraint, ctx, app)
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.icon.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        if !self.reduced_motion {
            let phase = self.clock.elapsed().as_secs_f32() * std::f32::consts::TAU / 2.;
            self.icon = self.icon.with_opacity(0.75 + 0.25 * phase.cos());
            ctx.repaint_after(Duration::from_millis(66));
        }
        self.icon.paint(origin, ctx, app);
    }

    fn size(&self) -> Option<Vector2F> {
        self.icon.size()
    }

    fn origin(&self) -> Option<Point> {
        self.icon.origin()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        self.icon.dispatch_event(event, ctx, app)
    }
}
