//! `CallDetails.tsx`: one HTTP call in the details pane, in place of the node while the HTTP tab is up (docs/specs/
//! 2026-10-25-http-inspector.md R4): its request, its response or the error of a call that got none, and Copy as cURL.
//!
//! What the pane reads is `grace_core::http_list::call_text`'s, worked out once for the call; this draws it. Headers,
//! bodies and the error are selectable text (migration guide R4), and Copy as cURL goes through the seams.
use gpui::accesskit::{Live, Role};
use gpui::{
    AnyElement, App, Entity, FontWeight, InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use gpui_base::SelectableText;
use grace_core::http_list::{CallText, Message, Response};
use grace_core::model::http::HttpCall;
use grace_core::state::CallRef;

use super::{part, status_tint};
use crate::assets::Icon;
use crate::details::{Details, format_millis};
use crate::elements::{ButtonKind, ButtonSize, Off as _, icon, icon_text_button};
use crate::theme::{METRICS, MONO, Theme};

pub const COPY: &str = "Copy as cURL";
pub const COPIED: &str = "Copied";
pub const REQUEST: &str = "Request";
pub const RESPONSE: &str = "Response";

/// Copy as cURL's icon and words: the check and "Copied" for the two seconds after a copy.
pub fn copy_label(copied: bool) -> (Icon, &'static str) {
    if copied {
        (Icon::Check, COPIED)
    } else {
        (Icon::Copy, COPY)
    }
}

/// A part of the pane: found by the end-to-end tests by its debug selector, read by assistive technology by its role
/// and its value (a `Label` reads `aria_value`).
fn labelled(id: &'static str, at: &CallRef, text: &str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(part(id, at))
        .debug_selector(move || id.into())
        .role(Role::Label)
        .aria_value(SharedString::from(text.to_owned()))
}

/// Text that can be selected and copied: its own selection per call, so a selection does not outlive the call.
fn selectable(id: &'static str, at: &CallRef, text: &str) -> SelectableText {
    SelectableText::new(part(id, at), text.to_owned())
}

fn section_title(id: &'static str, title: &'static str, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    div()
        .id(id)
        .debug_selector(move || id.into())
        .role(Role::Heading)
        .aria_level(3)
        .aria_label(title)
        .text_size(px(METRICS.text_xs.size))
        .font_weight(FontWeight::MEDIUM)
        .text_color(Theme::color(t.muted))
        .child(title)
        .into_any_element()
}

/// A message's headers and body, each in a sunken box that scrolls past 320 px, as `BOX` of `CallDetails.tsx`.
fn message(ids: [&'static str; 2], at: &CallRef, m: &Message, cx: &App) -> Vec<AnyElement> {
    let t = Theme::global(cx).tokens;
    let boxed = |id: &'static str, text: &str| {
        labelled(id, at, text)
            .w_full()
            .max_h(px(320.))
            .overflow_scroll()
            .rounded(px(METRICS.radius_lg))
            .bg(Theme::color(t.sunken))
            .p_2()
            .font_family(MONO)
            .text_size(px(METRICS.text_xs.size))
            .whitespace_nowrap()
            .child(selectable(id, at, text))
            .into_any_element()
    };
    [(ids[0], &m.headers), (ids[1], &m.body)]
        .into_iter()
        .filter_map(|(id, text)| text.as_deref().map(|text| boxed(id, text)))
        .collect()
}

/// The error of a call that got no response, in a failed box under Response.
fn failed(at: &CallRef, error: &str, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    div()
        .id("call-error-box")
        .debug_selector(|| "call-error-box".into())
        .role(Role::Group)
        .aria_label(crate::details::ERROR)
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded(px(METRICS.radius_lg))
        .bg(Theme::color(t.failed_tint))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(px(METRICS.text_sm.size))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(Theme::color(t.failed))
                .child(icon(Icon::CircleAlert, 14., Theme::color(t.failed)))
                .child(crate::details::ERROR),
        )
        .child(
            labelled("call-error", at, error)
                .font_family(MONO)
                .text_size(px(METRICS.text_xs.size))
                .text_color(Theme::color(t.ink))
                .child(selectable("call-error", at, error)),
        )
        .into_any_element()
}

/// The pane for the call at `at`, one box of its parts. `copied` is whether Copy as cURL was pressed on this call in the last two seconds.
pub fn view(
    this: &Entity<Details>,
    at: &CallRef,
    call: &HttpCall,
    text: &CallText,
    copied: bool,
    cx: &App,
) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let heading = div()
        .id("call-heading")
        .role(Role::Heading)
        .aria_level(2)
        .aria_label(SharedString::from(match &call.url {
            Some(url) => format!("{} {url}", call.method),
            None => call.method.clone(),
        }))
        .flex()
        .gap_2()
        .font_family(MONO)
        .text_size(px(METRICS.text_lg.size))
        .font_weight(FontWeight::MEDIUM)
        .child(
            div()
                .debug_selector(|| "call-method".into())
                .flex_none()
                .child(selectable("call-method", at, &call.method)),
        )
        .children(call.url.as_ref().map(|url| {
            div()
                .debug_selector(|| "call-url".into())
                .min_w_0()
                .child(selectable("call-url", at, url))
        }));

    let (fill, ink) = status_tint(call.status).colours(t);
    let (code, group) = text.status.split_at(text.code_len);
    let status = div()
        .flex()
        .items_center()
        .gap_3()
        .text_size(px(METRICS.text_sm.size))
        // room for the group's name here, where a list row only has its tooltip (R7.2)
        .child(
            div()
                .id("call-status")
                .debug_selector(|| "call-status".into())
                .role(Role::Label)
                .aria_value(SharedString::from(text.status.clone()))
                .flex()
                .items_center()
                .h(px(22.))
                .px_2()
                .rounded_full()
                .bg(Theme::color(fill))
                .text_color(Theme::color(ink))
                .text_size(px(METRICS.text_xs.size))
                .font_weight(FontWeight::MEDIUM)
                .child(div().font_family(MONO).child(code.to_owned()))
                .child(group.to_owned()),
        )
        .children(call.millis.map(|ms| {
            div()
                .debug_selector(|| "call-time".into())
                .font_family(MONO)
                .text_size(px(METRICS.text_xs.size))
                .text_color(Theme::color(t.muted))
                .child(format_millis(ms))
        }));

    let muted = Theme::color(t.muted);
    let whose = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(METRICS.text_sm.size))
                .text_color(muted)
                .child(selectable("call-test", at, &text.test)),
        )
        .child(
            labelled("call-location", at, &text.location)
                .font_family(MONO)
                .text_size(px(METRICS.text_xs.size))
                .text_color(muted)
                .child(selectable("call-location", at, &text.location)),
        )
        .children(text.via.as_ref().map(|via| {
            div()
                .debug_selector(|| "call-via".into())
                .text_size(px(METRICS.text_xs.size))
                .text_color(muted)
                .child(via.clone())
        }));

    let (glyph, label) = copy_label(copied);
    let curl = text.curl.clone();
    let (me, copy_at) = (this.clone(), at.clone());
    let button = icon_text_button("copy-curl", glyph, label, ButtonKind::Plain, ButtonSize::Small)
        .debug_selector(|| "copy-curl".into())
        .off(curl.is_none())
        .on_click(move |_, _, cx| {
            if let Some(curl) = curl.clone() {
                let at = copy_at.clone();
                me.update(cx, |d, cx| d.copy(at, curl, cx));
            }
        });
    // a live region that is always there: the label it holds is announced as it changes ("Copied")
    let copy = crate::a11y::live(
        div()
            .id("copy-status")
            .role(Role::Status)
            .aria_label(label)
            .flex()
            .child(button),
        Live::Polite,
    );

    let mut out = vec![
        heading.into_any_element(),
        status.into_any_element(),
        whose.into_any_element(),
        copy.into_any_element(),
    ];
    if let Some(request) = &text.request {
        out.push(section_title("call-request-title", REQUEST, cx));
        out.extend(message(["call-request-headers", "call-request-body"], at, request, cx));
    }
    if let Some(response) = &text.response {
        out.push(section_title("call-response-title", RESPONSE, cx));
        match response {
            Response::Received(m) => out.extend(message(["call-response-headers", "call-response-body"], at, m, cx)),
            Response::Failed(error) => out.push(failed(at, error, cx)),
        }
    }
    div()
        .id("call-details")
        .debug_selector(|| "call-details".into())
        .flex()
        .flex_col()
        .gap_3()
        .children(out)
        .into_any_element()
}

#[cfg(test)]
mod tests;
