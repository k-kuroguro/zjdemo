use std::collections::BTreeMap;

use colored::Colorize;
use handlebars::{RenderError, RenderErrorReason};
use zellij_tile::prelude::*;

mod formatter;

use crate::formatter::Formatter;

#[derive(Default)]
struct State {
   session_infos: Vec<SessionInfo>,
   formatter: Formatter,
}

#[derive(Copy, Clone)]
enum Mode {
   ListSessions,
   ListTabs,
}

impl Mode {
   fn from_pipe_name(name: &str) -> Option<Self> {
      match name {
         "list-sessions" => Some(Mode::ListSessions),
         "list-tabs" => Some(Mode::ListTabs),
         _ => None,
      }
   }

   fn default_format(&self) -> &str {
      match self {
         Mode::ListSessions => "{{session.name}}",
         Mode::ListTabs => "{{session.name}} - {{tab.name}}",
      }
   }
}

impl ZellijPlugin for State {
   fn load(&mut self, _configuration: BTreeMap<String, String>) {
      request_permission(&[
         PermissionType::ReadCliPipes,
         PermissionType::ReadApplicationState,
      ]);
      subscribe(&[EventType::SessionUpdate]);
   }

   fn update(&mut self, event: Event) -> bool {
      if let Event::SessionUpdate(session_infos, _) = event {
         self.session_infos = session_infos;
         return false;
      }

      false
   }

   fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
      let PipeSource::Cli(input_pipe_id) = pipe_message.source else {
         return false;
      };

      if self.session_infos.is_empty() {
         return false;
      }

      let args = pipe_message.args;
      let current_session_only = args.get("current-session").is_some();

      let mode = Mode::from_pipe_name(&pipe_message.name);
      let Some(mode) = mode else {
         return false;
      };

      let format = pipe_message
         .payload
         .unwrap_or_else(|| mode.default_format().to_string());
      self.output_formatted(&input_pipe_id, &format, mode, current_session_only);

      false
   }
}

impl State {
   fn output_formatted(&self, pipe_id: &str, format: &str, mode: Mode, current_session_only: bool) {
      let results: Result<Vec<String>, RenderError> = match mode {
         Mode::ListSessions => self
            .session_infos
            .iter()
            .map(|s| self.formatter.format(format, s, None))
            .collect(),
         Mode::ListTabs => self
            .session_infos
            .iter()
            .flat_map(|s| s.tabs.iter().map(move |t| (s, t)))
            .filter_map(|(s, t)| {
               if current_session_only && !s.is_current_session {
                  None
               } else {
                  Some(self.formatter.format(format, s, Some(t)))
               }
            })
            .collect(),
      };

      match results {
         Ok(lines) => {
            let output = lines.join("\n");
            cli_pipe_output(pipe_id, &format!("{}\n", output));
         }
         Err(e) => {
            cli_pipe_output(pipe_id, &self.format_error_message(format, &e));
         }
      }
   }

   fn format_error_message(&self, format: &str, error: &RenderError) -> String {
      let error_label = "error".bright_red().bold();

      match error.reason() {
         RenderErrorReason::TemplateError(te) => {
            if let Some((line, col)) = te.pos() {
               let lines: Vec<_> = format.lines().collect();
               let src_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");

               let sep = "|".bright_blue();
               let arrow = "-->".bright_blue();
               let line_num = format!("{line:>2}").bright_blue();
               let caret = "^".bright_red().bold();
               let spaces = " ".repeat(col.saturating_sub(1));

               format!(
                  "{error_label}: {}\n  {arrow} Format error in {line}:{col}\n   {sep}\n{line_num} {sep} {src_line}\n   {sep} {spaces}{caret}\n",
                  te.reason(),
               )
            } else {
               format!("{error_label}: {}\n", te.reason())
            }
         }
         _ => {
            format!("{error_label}: {}\n", error.reason())
         }
      }
   }
}

register_plugin!(State);
