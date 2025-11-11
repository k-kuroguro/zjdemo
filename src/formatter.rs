use std::sync::LazyLock;

use colored::Colorize;
use handlebars::{Handlebars, JsonRender, RenderError, handlebars_helper};
use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};
use zellij_tile::prelude::*;

static REGEX_BRIGHT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"bright_").unwrap());

handlebars_helper!(add: |a: usize, b: usize| a + b);
handlebars_helper!(sub: |a: usize, b: usize| a - b);
handlebars_helper!(mul: |a: usize, b: usize| a * b);
handlebars_helper!(div: |a: usize, b: usize| a / b);
handlebars_helper!(r#mod: |a: usize, b: usize| a % b);
handlebars_helper!(style: |text: Value, styles: str| {
   let mut result = text.render().normal();

   for s in styles.split(',') {
      let s = REGEX_BRIGHT.replace(s.trim(), "bright "); // The colored crate expects "bright red", but we prefer to use "bright_red" in our styles.

      result = match s.as_ref() {
         "bold" => result.bold(),
         "underline" => result.underline(),
         "italic" => result.italic(),
         "dimmed" => result.dimmed(),
         "reversed" => result.reversed(),
         "blink" => result.blink(),
         "hidden" => result.hidden(),
         "strikethrough" => result.strikethrough(),
         s if s.starts_with("on_") => result.on_color(s.trim_start_matches("on_")),
         _ => result.color(s.as_ref()),
      };
   }

   result.to_string()
});
handlebars_helper!(join: |{sep:str=","}, *args| args.iter().map(|a| a.render()).collect::<Vec<String>>().join(sep));

// Prevent errors on missing helpers
handlebars_helper!(helperMissing: |*_args| ());
handlebars_helper!(blockHelperMissing: |*_args| ());

macro_rules! register_helpers {
   ($reg:expr, $($name:expr => $helper:expr),* $(,)?) => {
      $(
         $reg.register_helper($name, Box::new($helper));
      )*
   };
}

#[derive(Serialize)]
struct Session {
   name: String,
   connected_clients: usize,
   is_current_session: bool,
   web_clients_allowed: bool,
   web_client_count: usize,
   tab_count: usize,
}

#[derive(Serialize)]
struct Tab {
   position: usize,
   name: String,
   active: bool,
   is_fullscreen_active: bool,
   is_sync_panes_active: bool,
   are_floating_panes_visible: bool,
   is_swap_layout_dirty: bool,
   viewport_rows: usize,
   viewport_columns: usize,
   display_area_rows: usize,
   display_area_columns: usize,
   selectable_tiled_panes_count: usize,
   selectable_floating_panes_count: usize,
   pane_count: usize,
}

impl Session {
   fn new(session_info: &SessionInfo) -> Self {
      Session {
         name: session_info.name.clone(),
         connected_clients: session_info.connected_clients,
         is_current_session: session_info.is_current_session,
         web_clients_allowed: session_info.web_clients_allowed,
         web_client_count: session_info.web_client_count,
         tab_count: session_info.tabs.len(),
      }
   }
}

impl Tab {
   fn new(tab_info: &TabInfo, pane_infos: &Vec<PaneInfo>) -> Self {
      Tab {
         position: tab_info.position,
         name: tab_info.name.clone(),
         active: tab_info.active,
         is_fullscreen_active: tab_info.is_fullscreen_active,
         is_sync_panes_active: tab_info.is_sync_panes_active,
         are_floating_panes_visible: tab_info.are_floating_panes_visible,
         is_swap_layout_dirty: tab_info.is_swap_layout_dirty,
         viewport_rows: tab_info.viewport_rows,
         viewport_columns: tab_info.viewport_columns,
         display_area_rows: tab_info.display_area_rows,
         display_area_columns: tab_info.display_area_columns,
         selectable_tiled_panes_count: tab_info.selectable_tiled_panes_count,
         selectable_floating_panes_count: tab_info.selectable_floating_panes_count,
         pane_count: pane_infos.len(),
      }
   }
}

pub(super) struct Formatter {
   reg: Handlebars<'static>,
}

impl Formatter {
   pub fn new() -> Self {
      let mut reg = Handlebars::new();

      register_helpers! {
         reg,
         "add" => add,
         "sub" => sub,
         "mul" => mul,
         "div" => div,
         "mod" => r#mod,
         "style" => style,
         "join" => join,
         "helperMissing" => helperMissing,
         "blockHelperMissing" => blockHelperMissing,
      }

      Formatter { reg }
   }

   pub fn format(
      &self,
      format: &str,
      session_info: &SessionInfo,
      tab_info: Option<&TabInfo>,
   ) -> Result<String, RenderError> {
      let data = if let Some(tab_info) = tab_info {
         let pane_infos = &session_info.panes.panes[&tab_info.position];
         json!({
            "session": Session::new(session_info),
            "tab": Tab::new(tab_info, pane_infos),
         })
      } else {
         json!({
            "session": Session::new(session_info),
         })
      };

      self.reg.render_template(format, &data)
   }
}

impl Default for Formatter {
   fn default() -> Self {
      Self::new()
   }
}
