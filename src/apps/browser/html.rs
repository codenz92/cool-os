struct HtmlRenderState {
    link: Option<String>,
    kind: BrowserLineKind,
    pending_prefix: Option<String>,
    preformatted: bool,
    css_pre_stack: Vec<String>,
    suppress_text_stack: Vec<String>,
    skip_until: Option<String>,
    quote_depth: usize,
    list_depth: usize,
    ordered_stack: Vec<usize>,
    in_table: bool,
    in_table_cell: bool,
    table_cell_is_header: bool,
    table_cell_text: String,
    table_row: Vec<TableCell>,
    form_action: Option<String>,
    form_fields: Vec<FormField>,
    cell_has_form_control: bool,
    cell_form_link: Option<String>,
    cell_link: Option<String>,
    cell_controls: Vec<(usize, BrowserControl)>,
    align_stack: Vec<(String, BrowserAlign)>,
    style_stack: Vec<(String, BrowserLineStyle)>,
    list_style_stack: Vec<(String, CssListStyle)>,
    table_cell_align: BrowserAlign,
    open_elements: Vec<String>,
}

impl HtmlRenderState {
    fn new() -> Self {
        Self {
            link: None,
            kind: BrowserLineKind::Text,
            pending_prefix: None,
            preformatted: false,
            css_pre_stack: Vec::new(),
            suppress_text_stack: Vec::new(),
            skip_until: None,
            quote_depth: 0,
            list_depth: 0,
            ordered_stack: Vec::new(),
            in_table: false,
            in_table_cell: false,
            table_cell_is_header: false,
            table_cell_text: String::new(),
            table_row: Vec::new(),
            form_action: None,
            form_fields: Vec::new(),
            cell_has_form_control: false,
            cell_form_link: None,
            cell_link: None,
            cell_controls: Vec::new(),
            align_stack: Vec::new(),
            style_stack: Vec::new(),
            list_style_stack: Vec::new(),
            table_cell_align: BrowserAlign::Left,
            open_elements: Vec::new(),
        }
    }

    fn current_align(&self) -> BrowserAlign {
        self.align_stack
            .last()
            .map(|(_, align)| *align)
            .unwrap_or(BrowserAlign::Left)
    }

    fn push_align(&mut self, name: &str, align: BrowserAlign) {
        self.align_stack.push((String::from(name), align));
    }

    fn pop_align(&mut self, name: &str) {
        if let Some(pos) = self
            .align_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.align_stack.truncate(pos);
        }
    }

    fn current_line_style(&self) -> BrowserLineStyle {
        let mut out = BrowserLineStyle::default();
        for (_, style) in &self.style_stack {
            out = out.merged(*style);
        }
        out
    }

    fn push_style(&mut self, name: &str, style: BrowserLineStyle) {
        if !style.is_default() {
            self.style_stack.push((String::from(name), style));
        }
    }

    fn pop_style(&mut self, name: &str) {
        if let Some(pos) = self
            .style_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.style_stack.truncate(pos);
        }
    }

    fn current_list_style(&self) -> CssListStyle {
        self.list_style_stack
            .last()
            .map(|(_, style)| *style)
            .unwrap_or_else(|| {
                if self.ordered_stack.is_empty() {
                    CssListStyle::Disc
                } else {
                    CssListStyle::Decimal
                }
            })
    }

    fn push_list_style(&mut self, name: &str, style: CssListStyle) {
        self.list_style_stack.push((String::from(name), style));
    }

    fn pop_list_style(&mut self, name: &str) {
        if let Some(pos) = self
            .list_style_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.list_style_stack.truncate(pos);
        }
    }

    fn push_open_element(&mut self, name: &str) {
        if self.open_elements.len() < 64 {
            self.open_elements.push(String::from(name));
        }
    }

    fn pop_open_element(&mut self, name: &str) {
        if let Some(pos) = self
            .open_elements
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.open_elements.truncate(pos);
        }
    }

    fn has_open_element(&self, name: &str) -> bool {
        self.open_elements.iter().any(|tag_name| tag_name == name)
    }

    fn push_pre_style(&mut self, name: &str) {
        self.css_pre_stack.push(String::from(name));
    }

    fn pop_pre_style(&mut self, name: &str) {
        if let Some(pos) = self
            .css_pre_stack
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.css_pre_stack.truncate(pos);
        }
    }

    fn push_suppressed_text(&mut self, name: &str) {
        self.suppress_text_stack.push(String::from(name));
    }

    fn pop_suppressed_text(&mut self, name: &str) {
        if let Some(pos) = self
            .suppress_text_stack
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.suppress_text_stack.truncate(pos);
        }
    }

    fn suppresses_text(&self) -> bool {
        !self.suppress_text_stack.is_empty()
    }

    fn is_preformatted(&self) -> bool {
        self.preformatted || !self.css_pre_stack.is_empty()
    }
}

struct TableCell {
    text: String,
    header: bool,
    link: Option<String>,
    is_form_row: bool,
    align: BrowserAlign,
    controls: Vec<(usize, BrowserControl)>,
}

struct FormField {
    name: String,
    value: String,
}

impl BrowserDocumentState {
    fn from_html(base_url: &str, response: &str) -> Self {
        Self::from_html_with_external_css(base_url, response, Vec::new())
    }

    fn from_html_with_external_css(
        base_url: &str,
        response: &str,
        external_css: Vec<String>,
    ) -> Self {
        Self::from_html_with_external_css_and_scripts(
            base_url,
            response,
            external_css,
            Vec::new(),
            BrowserScriptStats::default(),
        )
    }

    fn from_html_with_external_css_and_scripts(
        base_url: &str,
        response: &str,
        external_css: Vec<String>,
        scripts: Vec<String>,
        script_stats: BrowserScriptStats,
    ) -> Self {
        let body = response_body_text(response).unwrap_or(response);
        let effective_base = extract_base_href(body, base_url);
        let mut state = Self {
            base_url: effective_base,
            source: String::from(body),
            external_css,
            dom: BrowserDomDocument::new(),
            forms: Vec::new(),
            controls: Vec::new(),
            script_handlers: Vec::new(),
            session_storage: Vec::new(),
            script_globals: Vec::new(),
            script_stats,
            pending_navigation: None,
            focused_control: None,
        };
        scan_dom_and_controls(body, &state.base_url.clone(), &mut state);
        state.finalize_select_values();
        state.collect_inline_event_handlers();
        state.execute_script_sources(&scripts);
        state
    }

    fn dom_has_element(&self, wanted: &str) -> bool {
        self.dom.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                BrowserDomNodeKind::Element { name, attrs }
                    if name == wanted
                        || attrs
                            .iter()
                            .any(|attr| attr.name == "id" && attr.value == wanted)
            )
        })
    }

    fn dom_text_contains(&self, needle: &str) -> bool {
        self.dom.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                BrowserDomNodeKind::Text(text) if text.contains(needle)
            )
        })
    }

    fn focus_control(&mut self, id: usize) -> bool {
        if self.controls.get(id).map(|c| c.disabled).unwrap_or(true) {
            return false;
        }
        self.focused_control = Some(id);
        true
    }

    fn focus_next_control(&mut self) -> bool {
        if self.controls.is_empty() {
            self.focused_control = None;
            return false;
        }
        let start = self
            .focused_control
            .map(|id| id.saturating_add(1))
            .unwrap_or(0);
        for offset in 0..self.controls.len() {
            let idx = (start + offset) % self.controls.len();
            if self.controls[idx].is_focusable() {
                self.focused_control = Some(idx);
                return true;
            }
        }
        false
    }

    fn edit_focused_control(&mut self, c: char) -> bool {
        let Some(id) = self.focused_control else {
            return false;
        };
        let Some(control) = self.controls.get_mut(id) else {
            return false;
        };
        let changed = match control.kind {
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => match c {
                '\u{8}' | '\u{7f}' => {
                    control.value.pop();
                    true
                }
                '\n' | '\r' if control.kind == BrowserFormControlKind::TextArea => {
                    if control.value.len() < MAX_FORM_VALUE {
                        control.value.push('\n');
                    }
                    true
                }
                _ if !c.is_control() && control.value.len() < MAX_FORM_VALUE => {
                    control.value.push(c);
                    true
                }
                _ => false,
            },
            BrowserFormControlKind::Checkbox if c == ' ' || c == '\n' || c == '\r' => {
                control.checked = !control.checked;
                true
            }
            BrowserFormControlKind::Radio if c == ' ' || c == '\n' || c == '\r' => {
                self.set_radio_checked(id);
                true
            }
            BrowserFormControlKind::Select if c == ' ' || c == '\n' || c == '\r' => {
                self.select_next_option(id);
                true
            }
            _ => false,
        };
        if changed {
            match self.controls.get(id).map(|control| control.kind) {
                Some(BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio) => {
                    self.sync_control_dom_checked(id);
                }
                Some(_) => self.sync_control_dom_value(id),
                None => {}
            }
            self.sync_source_from_dom();
        }
        changed
    }

    fn activate_control(&mut self, id: usize) -> BrowserControlActivation {
        if !self.focus_control(id) {
            return BrowserControlActivation::Ignored;
        }
        let Some(kind) = self.controls.get(id).map(|control| control.kind) else {
            return BrowserControlActivation::Ignored;
        };
        match kind {
            BrowserFormControlKind::Checkbox => {
                if let Some(control) = self.controls.get_mut(id) {
                    control.checked = !control.checked;
                }
                self.sync_control_dom_checked(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Radio => {
                self.set_radio_checked(id);
                self.sync_control_dom_checked(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Select => {
                self.select_next_option(id);
                self.sync_control_dom_value(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Submit | BrowserFormControlKind::Image => {
                let _ = self.run_control_event(id, BrowserScriptEvent::Click);
                let _ = self.run_control_event(id, BrowserScriptEvent::Submit);
                self.submission_for(id)
                    .unwrap_or(BrowserControlActivation::Ignored)
            }
            BrowserFormControlKind::Button => {
                let mutated = self.run_control_event(id, BrowserScriptEvent::Click);
                if mutated {
                    return BrowserControlActivation::Changed;
                }
                let label = self.controls[id].label.clone();
                BrowserControlActivation::DomEvent(label)
            }
            BrowserFormControlKind::Reset => {
                let _ = self.run_control_event(id, BrowserScriptEvent::Click);
                self.reset_form_for_control(id);
                self.sync_source_from_dom();
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => {
                BrowserControlActivation::Focused
            }
            BrowserFormControlKind::Hidden => BrowserControlActivation::Ignored,
        }
    }

    fn set_control_value_for_test(&mut self, name: &str, value: &str) -> bool {
        let Some(control) = self
            .controls
            .iter_mut()
            .find(|control| control.name == name && control.kind.accepts_text())
        else {
            return false;
        };
        control.value.clear();
        for c in value.chars().take(MAX_FORM_VALUE) {
            control.value.push(c);
        }
        true
    }

    fn toggle_control_for_test(&mut self, name: &str) -> bool {
        let Some(id) = self.controls.iter().position(|control| {
            control.name == name && control.kind == BrowserFormControlKind::Checkbox
        }) else {
            return false;
        };
        matches!(self.activate_control(id), BrowserControlActivation::Changed)
    }

    fn submit_url_for_test(&self, label: &str) -> Option<String> {
        let id = self
            .controls
            .iter()
            .position(|control| control.label == label && control.kind.can_submit())?;
        match self.submission_for(id)? {
            BrowserControlActivation::Navigate(url) => Some(url),
            BrowserControlActivation::Post { url, body } => {
                let mut out = String::from("POST ");
                out.push_str(&url);
                out.push_str(" body=");
                out.push_str(&body);
                Some(out)
            }
            _ => None,
        }
    }

    fn default_submit_for(&self, control_id: usize) -> Option<usize> {
        let form_id = self.controls.get(control_id)?.form_id?;
        self.controls
            .iter()
            .enumerate()
            .find(|(_, control)| {
                control.form_id == Some(form_id) && control.kind.can_submit() && !control.disabled
            })
            .map(|(id, _)| id)
    }

    fn submission_for(&self, submit_id: usize) -> Option<BrowserControlActivation> {
        let submit = self.controls.get(submit_id)?;
        let form_id = submit.form_id?;
        let form = self.forms.get(form_id)?;
        let body = self.encoded_form_body(form_id, Some(submit_id));
        match form.method {
            BrowserFormMethod::Get => {
                let mut url = form.action.clone();
                if !body.is_empty() {
                    url.push(if url.contains('?') { '&' } else { '?' });
                    url.push_str(&body);
                }
                Some(BrowserControlActivation::Navigate(url))
            }
            BrowserFormMethod::Post => Some(BrowserControlActivation::Post {
                url: form.action.clone(),
                body,
            }),
        }
    }

    fn encoded_form_body(&self, form_id: usize, submit_id: Option<usize>) -> String {
        let mut out = String::new();
        let mut wrote = false;
        for (idx, control) in self.controls.iter().enumerate() {
            if control.form_id != Some(form_id) || !control.successful(Some(idx) == submit_id) {
                continue;
            }
            if wrote {
                out.push('&');
            }
            push_query_encoded(&mut out, &control.name);
            out.push('=');
            push_query_encoded(&mut out, &control.submit_value());
            wrote = true;
        }
        out
    }

    fn set_radio_checked(&mut self, id: usize) {
        let Some(target) = self.controls.get(id).cloned() else {
            return;
        };
        let mut affected = Vec::new();
        for (idx, control) in self.controls.iter_mut().enumerate() {
            if control.kind == BrowserFormControlKind::Radio
                && control.form_id == target.form_id
                && !target.name.is_empty()
                && control.name == target.name
            {
                control.checked = false;
                affected.push(idx);
            }
        }
        if let Some(control) = self.controls.get_mut(id) {
            control.checked = true;
            if !affected.iter().any(|idx| *idx == id) {
                affected.push(id);
            }
        }
        for idx in affected {
            self.sync_control_dom_checked(idx);
        }
    }

    fn select_next_option(&mut self, id: usize) {
        let Some(control) = self.controls.get_mut(id) else {
            return;
        };
        if control.options.is_empty() {
            return;
        }
        control.selected = (control.selected + 1) % control.options.len();
        control.value = control.options[control.selected].value.clone();
    }

    fn reset_form_for_control(&mut self, id: usize) {
        let Some(form_id) = self.controls.get(id).and_then(|control| control.form_id) else {
            return;
        };
        for control in &mut self.controls {
            if control.form_id == Some(form_id) {
                control.reset_to_default();
            }
        }
    }

    fn finalize_select_values(&mut self) {
        for control in &mut self.controls {
            if control.kind == BrowserFormControlKind::Select && !control.options.is_empty() {
                control.selected = control.selected.min(control.options.len() - 1);
                control.value = control.options[control.selected].value.clone();
            }
        }
    }

    fn collect_inline_event_handlers(&mut self) {
        let mut handlers = Vec::new();
        for (node_id, node) in self.dom.nodes.iter().enumerate() {
            let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                continue;
            };
            for attr in attrs {
                let Some(event) = BrowserScriptEvent::from_attr(&attr.name) else {
                    continue;
                };
                if attr.value.trim().is_empty() || handlers.len() >= MAX_SCRIPT_EVENT_HANDLERS {
                    continue;
                }
                handlers.push(BrowserScriptHandler {
                    node_id,
                    event,
                    code: attr.value.clone(),
                });
            }
        }
        for handler in handlers {
            self.add_script_handler(handler.node_id, handler.event, handler.code);
        }
    }

    fn add_script_handler(&mut self, node_id: usize, event: BrowserScriptEvent, code: String) {
        if self.script_handlers.len() >= MAX_SCRIPT_EVENT_HANDLERS {
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
            return;
        }
        self.script_handlers.push(BrowserScriptHandler {
            node_id,
            event,
            code,
        });
        self.script_stats.handlers = self.script_stats.handlers.saturating_add(1);
    }

    fn execute_script_sources(&mut self, scripts: &[String]) {
        let before = self.script_stats.mutations;
        for script in scripts.iter().take(MAX_SCRIPT_SUBRESOURCES) {
            self.execute_script(script, 0);
        }
        if self.script_stats.mutations != before {
            self.sync_source_from_dom();
        }
    }

    fn execute_script(&mut self, code: &str, depth: usize) {
        self.execute_script_with_vars(code, depth, &[]);
    }

    fn execute_script_with_vars(&mut self, code: &str, depth: usize, vars: &[BrowserScriptVar]) {
        if depth > MAX_SCRIPT_RECURSION {
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
            return;
        }
        let statements = split_script_statements(code);
        for statement in statements.into_iter().take(MAX_SCRIPT_STATEMENTS) {
            if self.script_stats.statements >= MAX_SCRIPT_STATEMENTS {
                self.script_stats.errors = self.script_stats.errors.saturating_add(1);
                break;
            }
            let statement = statement.trim();
            if statement.is_empty() || script_statement_is_ignorable(statement) {
                continue;
            }
            self.script_stats.statements = self.script_stats.statements.saturating_add(1);
            if self.execute_script_statement(statement, depth, vars) {
                continue;
            }
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
        }
    }

    fn execute_script_statement(
        &mut self,
        statement: &str,
        depth: usize,
        vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(statement);
        if compact.starts_with("setTimeout(") || compact.starts_with("window.setTimeout(") {
            let Some(body) = extract_script_function_body(statement) else {
                return false;
            };
            self.script_stats.timers = self.script_stats.timers.saturating_add(1);
            self.execute_script_with_vars(&body, depth.saturating_add(1), vars);
            return true;
        }
        if self.execute_var_assignment(statement, vars) {
            return true;
        }
        if self.execute_fetch_statement(statement, depth, vars) {
            return true;
        }
        if self.execute_storage_statement(statement, vars) {
            return true;
        }
        if self.execute_class_list_statement(statement) {
            return true;
        }
        if self.execute_attribute_statement(statement, vars) {
            return true;
        }
        if self.execute_history_statement(statement, vars) {
            return true;
        }
        if let Some((target, event, body)) = parse_add_event_listener(statement) {
            let Some(node_id) = self.resolve_script_target(&target) else {
                return false;
            };
            self.add_script_handler(node_id, event, body);
            return true;
        }
        if let Some((left, right)) = split_script_assignment(statement) {
            if self.apply_global_script_assignment(left, right, vars) {
                return true;
            }
            let Some((target, property)) = parse_script_assignment_left(left) else {
                return false;
            };
            let Some(node_id) = self.resolve_script_target(&target) else {
                return false;
            };
            return self.apply_script_property(node_id, property, right, vars);
        }
        false
    }

    fn execute_var_assignment(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let trimmed = statement.trim();
        let rest = if let Some(rest) = trimmed.strip_prefix("var ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("let ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("const ") {
            rest
        } else {
            return false;
        };
        let Some((name, value_expr)) = split_script_assignment(rest) else {
            return false;
        };
        let name = name.trim();
        if !valid_script_var_name(name) {
            return false;
        }
        let Some(value) = self.script_string_value(value_expr, vars) else {
            return false;
        };
        self.set_script_global(name, &value);
        true
    }

    fn execute_fetch_statement(
        &mut self,
        statement: &str,
        depth: usize,
        _vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(statement);
        if !(compact.starts_with("fetch(") || compact.starts_with("window.fetch(")) {
            return false;
        }
        let Some(request) = parse_fetch_request(&compact) else {
            return false;
        };
        let url = resolve_url(&self.base_url, &request.url);
        if !script_url_allowed(&self.base_url, &url) {
            return false;
        }
        let resource = match request.method {
            BrowserFetchMethod::Get => load_subresource_uncached(&url, BrowserResourceKind::Script),
            BrowserFetchMethod::Post => load_fetch_post_uncached(&url, &request.body),
        };
        let Ok(resource) = resource else {
            return false;
        };
        if resource.bytes.len() > MAX_SCRIPT_FETCH_BYTES {
            return false;
        }
        let body = truncate_script_value(&String::from_utf8_lossy(&resource.bytes));
        self.script_stats.fetches = self.script_stats.fetches.saturating_add(1);
        if let Some(callback) = extract_fetch_text_callback_body(statement)
            .or_else(|| extract_script_function_body(statement))
        {
            let vars = vec![
                BrowserScriptVar {
                    name: String::from("text"),
                    value: body.clone(),
                },
                BrowserScriptVar {
                    name: String::from("body"),
                    value: body.clone(),
                },
                BrowserScriptVar {
                    name: String::from("responseText"),
                    value: body,
                },
            ];
            self.execute_script_with_vars(&callback, depth.saturating_add(1), &vars);
        }
        true
    }

    fn execute_storage_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_storage_call(&compact) else {
            return false;
        };
        match call.method {
            BrowserStorageMethod::SetItem => {
                let Some(key) = call.key else {
                    return false;
                };
                let Some(value_expr) = call.value_expr else {
                    return false;
                };
                let Some(value) = self.script_string_value(&value_expr, vars) else {
                    return false;
                };
                if self.set_script_storage(call.area, &key, &value) {
                    self.script_stats.storage_writes =
                        self.script_stats.storage_writes.saturating_add(1);
                    return true;
                }
            }
            BrowserStorageMethod::RemoveItem => {
                let Some(key) = call.key else {
                    return false;
                };
                if self.remove_script_storage(call.area, &key) {
                    self.script_stats.storage_writes =
                        self.script_stats.storage_writes.saturating_add(1);
                    return true;
                }
            }
            BrowserStorageMethod::Clear => {
                let changed = self.clear_script_storage(call.area);
                self.script_stats.storage_writes = self
                    .script_stats
                    .storage_writes
                    .saturating_add(changed.max(1));
                return true;
            }
        }
        false
    }

    fn execute_class_list_statement(&mut self, statement: &str) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_class_list_call(&compact) else {
            return false;
        };
        let Some(node_id) = self.resolve_script_target(&call.target) else {
            return false;
        };
        if self.mutate_node_class_list(node_id, &call.class_name, call.op) {
            self.note_script_mutation();
            return true;
        }
        false
    }

    fn execute_attribute_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_attribute_call(&compact) else {
            return false;
        };
        let Some(node_id) = self.resolve_script_target(&call.target) else {
            return false;
        };
        match call.op {
            BrowserAttributeOp::Set => {
                let Some(value_expr) = call.value_expr else {
                    return false;
                };
                let Some(value) = self.script_string_value(&value_expr, vars) else {
                    return false;
                };
                if self.set_node_attribute_from_script(node_id, &call.name, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserAttributeOp::Remove => {
                if self.remove_node_attribute_from_script(node_id, &call.name) {
                    self.note_script_mutation();
                    return true;
                }
            }
        }
        false
    }

    fn execute_history_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(url_expr) = parse_history_url_arg(&compact) else {
            return false;
        };
        let Some(url) = self.script_string_value(&url_expr, vars) else {
            return false;
        };
        let url = resolve_url(&self.base_url, &url);
        self.base_url = url;
        self.script_stats.navigation_requests =
            self.script_stats.navigation_requests.saturating_add(1);
        true
    }

    fn apply_global_script_assignment(
        &mut self,
        left: &str,
        right: &str,
        vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(left);
        if compact == "document.cookie" {
            let Some(cookie) = self.script_string_value(right, vars) else {
                return false;
            };
            let Some((scheme, host, path)) = self.cookie_context() else {
                return false;
            };
            if crate::browser_session::set_document_cookie_for_context(
                &scheme, &host, &path, &cookie,
            ) {
                self.script_stats.cookie_writes = self.script_stats.cookie_writes.saturating_add(1);
                return true;
            }
            return false;
        }
        if matches!(
            compact.as_str(),
            "location.href" | "window.location.href" | "document.location.href" | "window.location"
        ) {
            let Some(url) = self.script_string_value(right, vars) else {
                return false;
            };
            self.pending_navigation = Some(resolve_url(&self.base_url, &url));
            self.script_stats.navigation_requests =
                self.script_stats.navigation_requests.saturating_add(1);
            return true;
        }
        false
    }

    fn resolve_script_target(&self, target: &BrowserScriptTarget) -> Option<usize> {
        match target {
            BrowserScriptTarget::Id(id) => self.find_node_by_id(id),
            BrowserScriptTarget::Selector(selector) => self.query_selector(selector),
            BrowserScriptTarget::SelectorAll(selector, index) => {
                self.query_selector_all(selector).get(*index).copied()
            }
        }
    }

    fn find_node_by_id(&self, wanted: &str) -> Option<usize> {
        self.dom
            .nodes
            .iter()
            .enumerate()
            .find_map(|(node_id, node)| {
                let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                    return None;
                };
                attrs
                    .iter()
                    .any(|attr| attr.name == "id" && attr.value == wanted)
                    .then_some(node_id)
            })
    }

    fn query_selector(&self, selector: &str) -> Option<usize> {
        self.query_selector_all(selector).into_iter().next()
    }

    fn query_selector_all(&self, selector: &str) -> Vec<usize> {
        let selector = selector.trim();
        if selector.is_empty() {
            return Vec::new();
        }
        if let Some(id) = selector.strip_prefix('#') {
            return self.find_node_by_id(id).into_iter().collect();
        }
        if let Some(class) = selector.strip_prefix('.') {
            return self
                .dom
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(node_id, node)| {
                    let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                        return None;
                    };
                    attrs
                        .iter()
                        .find(|attr| attr.name == "class")
                        .map(|attr| attr.value.split_whitespace().any(|value| value == class))
                        .unwrap_or(false)
                        .then_some(node_id)
                })
                .collect();
        }
        let wanted = lowercase_ascii(selector);
        self.dom
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(node_id, node)| {
                matches!(
                    &node.kind,
                    BrowserDomNodeKind::Element { name, .. } if name == &wanted
                )
                .then_some(node_id)
            })
            .collect()
    }

    fn apply_script_property(
        &mut self,
        node_id: usize,
        property: BrowserScriptProperty,
        value: &str,
        vars: &[BrowserScriptVar],
    ) -> bool {
        match property {
            BrowserScriptProperty::TextContent => {
                let Some(text) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_text_content(node_id, &text) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::ClassName => {
                let Some(class_name) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_attr(node_id, "class", &class_name) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Value => {
                let Some(value) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_value(node_id, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Checked => {
                let Some(checked) = parse_script_bool_value(value) else {
                    return false;
                };
                if self.set_node_checked(node_id, checked) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Disabled => {
                let Some(disabled) = parse_script_bool_value(value) else {
                    return false;
                };
                if self.set_node_disabled(node_id, disabled) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Style(property) => {
                let Some(value) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_style_property(node_id, &property, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
        }
        false
    }

    fn script_string_value(&mut self, input: &str, vars: &[BrowserScriptVar]) -> Option<String> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        if let Some(parts) = split_script_concat(input) {
            let mut out = String::new();
            for part in parts {
                let value = self.script_string_value(&part, vars)?;
                if out.len().saturating_add(value.len()) > MAX_FORM_VALUE {
                    let remaining = MAX_FORM_VALUE.saturating_sub(out.len());
                    out.push_str(&value[..value.len().min(remaining)]);
                    break;
                }
                out.push_str(&value);
            }
            return Some(out);
        }
        if let Some(value) = parse_script_string_value(input) {
            return Some(value);
        }
        let compact = compact_script_expr(input);
        if compact == "null" || compact == "undefined" {
            return Some(String::new());
        }
        if compact == "document.cookie" {
            let Some((scheme, host, path)) = self.cookie_context() else {
                return Some(String::new());
            };
            self.script_stats.cookie_reads = self.script_stats.cookie_reads.saturating_add(1);
            return Some(crate::browser_session::document_cookie_for_context(
                &scheme, &host, &path,
            ));
        }
        if matches!(
            compact.as_str(),
            "location.href" | "window.location.href" | "document.location.href"
        ) {
            return Some(self.base_url.clone());
        }
        if matches!(
            compact.as_str(),
            "location.search" | "window.location.search" | "document.location.search"
        ) {
            return Some(location_search(&self.base_url));
        }
        if let Some(value) = self.storage_get_from_expr(&compact) {
            return Some(value);
        }
        if let Some(value) = self.node_get_attribute_from_expr(&compact) {
            return Some(value);
        }
        if let Some(value) = self.node_property_value_from_expr(&compact) {
            return Some(value);
        }
        for var in vars.iter().take(MAX_SCRIPT_VARS) {
            if var.name == compact {
                return Some(truncate_script_value(&var.value));
            }
        }
        for var in self.script_globals.iter().take(MAX_SCRIPT_VARS) {
            if var.name == compact {
                return Some(truncate_script_value(&var.value));
            }
        }
        None
    }

    fn set_script_global(&mut self, name: &str, value: &str) {
        if let Some(var) = self.script_globals.iter_mut().find(|var| var.name == name) {
            var.value = truncate_script_value(value);
            return;
        }
        if self.script_globals.len() >= MAX_SCRIPT_VARS {
            self.script_globals.remove(0);
        }
        self.script_globals.push(BrowserScriptVar {
            name: String::from(name),
            value: truncate_script_value(value),
        });
    }

    fn storage_get_from_expr(&mut self, compact: &str) -> Option<String> {
        let (area, key) = parse_storage_get_item_expr(compact)?;
        let value = self
            .get_script_storage(area, &key)
            .unwrap_or_else(String::new);
        self.script_stats.storage_reads = self.script_stats.storage_reads.saturating_add(1);
        Some(value)
    }

    fn node_get_attribute_from_expr(&self, compact: &str) -> Option<String> {
        let call = parse_get_attribute_expr(compact)?;
        let node_id = self.resolve_script_target(&call.target)?;
        self.node_attr_value(node_id, &call.name)
            .or_else(|| Some(String::new()))
    }

    fn node_property_value_from_expr(&self, compact: &str) -> Option<String> {
        let (target, property) = parse_script_assignment_left(compact)?;
        let node_id = self.resolve_script_target(&target)?;
        match property {
            BrowserScriptProperty::TextContent => Some(self.node_text_content(node_id)),
            BrowserScriptProperty::ClassName => self
                .node_attr_value(node_id, "class")
                .or_else(|| Some(String::new())),
            BrowserScriptProperty::Value => Some(self.node_value(node_id)),
            BrowserScriptProperty::Checked => Some(
                self.control_for_node(node_id)
                    .and_then(|control_id| self.controls.get(control_id))
                    .map(|control| if control.checked { "true" } else { "false" })
                    .unwrap_or("false")
                    .into(),
            ),
            BrowserScriptProperty::Disabled => Some(
                self.control_for_node(node_id)
                    .and_then(|control_id| self.controls.get(control_id))
                    .map(|control| if control.disabled { "true" } else { "false" })
                    .unwrap_or("false")
                    .into(),
            ),
            BrowserScriptProperty::Style(property) => Some(
                self.node_style_property(node_id, &property)
                    .unwrap_or_else(String::new),
            ),
        }
    }

    fn cookie_context(&self) -> Option<(String, String, String)> {
        parse_web_url(&self.base_url).ok()
    }

    fn note_script_mutation(&mut self) {
        self.script_stats.mutations = self.script_stats.mutations.saturating_add(1);
    }

    fn set_node_text_content(&mut self, node_id: usize, text: &str) -> bool {
        if node_id >= self.dom.nodes.len() {
            return false;
        }
        let text = truncate_script_value(text);
        let first_text_child = self.dom.nodes[node_id]
            .children
            .iter()
            .copied()
            .find(|child| {
                matches!(
                    self.dom.nodes.get(*child).map(|node| &node.kind),
                    Some(BrowserDomNodeKind::Text(_))
                )
            });
        if let Some(child) = first_text_child {
            if let Some(BrowserDomNode {
                kind: BrowserDomNodeKind::Text(existing),
                ..
            }) = self.dom.nodes.get_mut(child)
            {
                *existing = text;
            }
            if let Some(node) = self.dom.nodes.get_mut(node_id) {
                node.children.clear();
                node.children.push(child);
            }
            return true;
        }
        if self.dom.nodes.len() >= MAX_DOM_NODES {
            return false;
        }
        if let Some(node) = self.dom.nodes.get_mut(node_id) {
            node.children.clear();
        }
        self.dom.push_text_raw(node_id, text);
        true
    }

    fn node_text_content(&self, node_id: usize) -> String {
        let mut out = String::new();
        self.push_node_text_content(node_id, &mut out);
        truncate_script_value(&out)
    }

    fn push_node_text_content(&self, node_id: usize, out: &mut String) {
        let Some(node) = self.dom.nodes.get(node_id) else {
            return;
        };
        match &node.kind {
            BrowserDomNodeKind::Text(text) => out.push_str(text),
            BrowserDomNodeKind::Element { .. } => {
                for child in &node.children {
                    self.push_node_text_content(*child, out);
                    if out.len() >= MAX_FORM_VALUE {
                        out.truncate(MAX_FORM_VALUE);
                        break;
                    }
                }
            }
        }
    }

    fn set_node_value(&mut self, node_id: usize, value: &str) -> bool {
        let value = truncate_script_value(value);
        if let Some(control_id) = self.control_for_node(node_id) {
            self.set_control_value(control_id, &value);
            return true;
        }
        self.set_node_attr(node_id, "value", &value)
    }

    fn node_value(&self, node_id: usize) -> String {
        if let Some(control_id) = self.control_for_node(node_id) {
            return self
                .controls
                .get(control_id)
                .map(|control| control.value.clone())
                .unwrap_or_else(String::new);
        }
        self.node_attr_value(node_id, "value")
            .unwrap_or_else(String::new)
    }

    fn set_node_checked(&mut self, node_id: usize, checked: bool) -> bool {
        let Some(control_id) = self.control_for_node(node_id) else {
            if checked {
                return self.set_node_attr(node_id, "checked", "");
            }
            return self.remove_node_attr(node_id, "checked");
        };
        if checked
            && self
                .controls
                .get(control_id)
                .map(|control| control.kind == BrowserFormControlKind::Radio)
                .unwrap_or(false)
        {
            self.set_radio_checked(control_id);
        } else if let Some(control) = self.controls.get_mut(control_id) {
            control.checked = checked;
        }
        self.sync_control_dom_checked(control_id);
        true
    }

    fn set_node_disabled(&mut self, node_id: usize, disabled: bool) -> bool {
        if let Some(control_id) = self.control_for_node(node_id) {
            if let Some(control) = self.controls.get_mut(control_id) {
                control.disabled = disabled;
            }
        }
        if disabled {
            self.set_node_attr(node_id, "disabled", "")
        } else {
            self.remove_node_attr(node_id, "disabled")
        }
    }

    fn set_node_attr(&mut self, node_id: usize, name: &str, value: &str) -> bool {
        let Some(BrowserDomNode {
            kind: BrowserDomNodeKind::Element { attrs, .. },
            ..
        }) = self.dom.nodes.get_mut(node_id)
        else {
            return false;
        };
        if let Some(attr) = attrs.iter_mut().find(|attr| attr.name == name) {
            attr.value = String::from(value);
            return true;
        }
        if attrs.len() >= MAX_DOM_ATTRS {
            return false;
        }
        attrs.push(BrowserDomAttr {
            name: String::from(name),
            value: String::from(value),
        });
        true
    }

    fn set_node_attribute_from_script(&mut self, node_id: usize, name: &str, value: &str) -> bool {
        let name = lowercase_ascii(name.trim());
        match name.as_str() {
            "class" => self.set_node_attr(node_id, "class", &truncate_script_value(value)),
            "style" => self.set_node_attr(node_id, "style", &truncate_script_value(value)),
            "value" => self.set_node_value(node_id, value),
            "checked" => self.set_node_checked(node_id, true),
            "disabled" => self.set_node_disabled(node_id, true),
            _ => self.set_node_attr(node_id, &name, &truncate_script_value(value)),
        }
    }

    fn remove_node_attr(&mut self, node_id: usize, name: &str) -> bool {
        let Some(BrowserDomNode {
            kind: BrowserDomNodeKind::Element { attrs, .. },
            ..
        }) = self.dom.nodes.get_mut(node_id)
        else {
            return false;
        };
        if let Some(pos) = attrs.iter().position(|attr| attr.name == name) {
            attrs.remove(pos);
        }
        true
    }

    fn remove_node_attribute_from_script(&mut self, node_id: usize, name: &str) -> bool {
        let name = lowercase_ascii(name.trim());
        match name.as_str() {
            "checked" => self.set_node_checked(node_id, false),
            "disabled" => self.set_node_disabled(node_id, false),
            _ => self.remove_node_attr(node_id, &name),
        }
    }

    fn node_attr_value(&self, node_id: usize, name: &str) -> Option<String> {
        let BrowserDomNodeKind::Element { attrs, .. } = &self.dom.nodes.get(node_id)?.kind else {
            return None;
        };
        attrs
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value.clone())
    }

    fn set_node_style_property(&mut self, node_id: usize, property: &str, value: &str) -> bool {
        let mut style = self
            .node_attr_value(node_id, "style")
            .unwrap_or_else(String::new);
        set_style_declaration(&mut style, property, &truncate_script_value(value));
        self.set_node_attr(node_id, "style", &style)
    }

    fn node_style_property(&self, node_id: usize, property: &str) -> Option<String> {
        let style = self.node_attr_value(node_id, "style")?;
        style_declaration_value(&style, property)
    }

    fn mutate_node_class_list(
        &mut self,
        node_id: usize,
        class_name: &str,
        op: ClassListOp,
    ) -> bool {
        if class_name.trim().is_empty() {
            return false;
        }
        let class_name = truncate_script_value(class_name.trim());
        let current = self
            .node_attr_value(node_id, "class")
            .unwrap_or_else(String::new);
        let mut classes: Vec<String> = current
            .split_whitespace()
            .map(String::from)
            .take(16)
            .collect();
        let exists = classes.iter().any(|class| class == &class_name);
        match op {
            ClassListOp::Add => {
                if !exists {
                    classes.push(class_name);
                }
            }
            ClassListOp::Remove => {
                classes.retain(|class| class != &class_name);
            }
            ClassListOp::Toggle => {
                if exists {
                    classes.retain(|class| class != &class_name);
                } else {
                    classes.push(class_name);
                }
            }
        }
        self.set_node_attr(node_id, "class", &classes.join(" "))
    }

    fn control_for_node(&self, node_id: usize) -> Option<usize> {
        self.controls
            .iter()
            .position(|control| control.dom_node == Some(node_id))
    }

    fn set_control_value(&mut self, control_id: usize, value: &str) {
        let Some(control) = self.controls.get_mut(control_id) else {
            return;
        };
        control.value = truncate_script_value(value);
        if control.kind == BrowserFormControlKind::Select {
            if let Some(pos) = control
                .options
                .iter()
                .position(|option| option.value == control.value || option.label == control.value)
            {
                control.selected = pos;
            }
        }
        self.sync_control_dom_value(control_id);
    }

    fn sync_control_dom_value(&mut self, control_id: usize) {
        let Some(control) = self.controls.get(control_id).cloned() else {
            return;
        };
        let Some(node_id) = control.dom_node else {
            return;
        };
        match control.kind {
            BrowserFormControlKind::Text
            | BrowserFormControlKind::Hidden
            | BrowserFormControlKind::Submit
            | BrowserFormControlKind::Button
            | BrowserFormControlKind::Reset
            | BrowserFormControlKind::Image
            | BrowserFormControlKind::Checkbox
            | BrowserFormControlKind::Radio => {
                let _ = self.set_node_attr(node_id, "value", &control.value);
            }
            BrowserFormControlKind::TextArea => {
                let _ = self.set_node_text_content(node_id, &control.value);
            }
            BrowserFormControlKind::Select => {
                for (idx, option_node) in self
                    .option_nodes_for_select(node_id)
                    .into_iter()
                    .enumerate()
                {
                    if idx == control.selected {
                        let _ = self.set_node_attr(option_node, "selected", "");
                    } else {
                        let _ = self.remove_node_attr(option_node, "selected");
                    }
                }
            }
        }
    }

    fn sync_control_dom_checked(&mut self, control_id: usize) {
        let Some(control) = self.controls.get(control_id).cloned() else {
            return;
        };
        let Some(node_id) = control.dom_node else {
            return;
        };
        if control.checked {
            let _ = self.set_node_attr(node_id, "checked", "");
        } else {
            let _ = self.remove_node_attr(node_id, "checked");
        }
    }

    fn option_nodes_for_select(&self, select_node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(node) = self.dom.nodes.get(select_node) else {
            return out;
        };
        for child in &node.children {
            if matches!(
                self.dom.nodes.get(*child).map(|node| &node.kind),
                Some(BrowserDomNodeKind::Element { name, .. }) if name == "option"
            ) {
                out.push(*child);
            }
        }
        out
    }

    fn get_script_storage(&self, area: BrowserStorageArea, key: &str) -> Option<String> {
        match area {
            BrowserStorageArea::Local => {
                crate::browser_storage::local_get(&storage_origin_for_url(&self.base_url)?, key)
            }
            BrowserStorageArea::Session => self
                .session_storage
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| entry.value.clone()),
        }
    }

    fn set_script_storage(&mut self, area: BrowserStorageArea, key: &str, value: &str) -> bool {
        match area {
            BrowserStorageArea::Local => {
                let Some(origin) = storage_origin_for_url(&self.base_url) else {
                    return false;
                };
                crate::browser_storage::local_set(&origin, key, value)
            }
            BrowserStorageArea::Session => {
                if key.is_empty() || key.len() > 64 || value.len() > MAX_FORM_VALUE {
                    return false;
                }
                if let Some(entry) = self
                    .session_storage
                    .iter_mut()
                    .find(|entry| entry.key == key)
                {
                    entry.value = truncate_script_value(value);
                    return true;
                }
                if self.session_storage.len() >= MAX_SESSION_STORAGE_ENTRIES {
                    self.session_storage.remove(0);
                }
                self.session_storage.push(BrowserSessionStorageEntry {
                    key: String::from(key),
                    value: truncate_script_value(value),
                });
                true
            }
        }
    }

    fn remove_script_storage(&mut self, area: BrowserStorageArea, key: &str) -> bool {
        match area {
            BrowserStorageArea::Local => {
                let Some(origin) = storage_origin_for_url(&self.base_url) else {
                    return false;
                };
                crate::browser_storage::local_remove(&origin, key)
            }
            BrowserStorageArea::Session => {
                let Some(pos) = self
                    .session_storage
                    .iter()
                    .position(|entry| entry.key == key)
                else {
                    return false;
                };
                self.session_storage.remove(pos);
                true
            }
        }
    }

    fn clear_script_storage(&mut self, area: BrowserStorageArea) -> usize {
        match area {
            BrowserStorageArea::Local => storage_origin_for_url(&self.base_url)
                .map(|origin| crate::browser_storage::local_clear(&origin))
                .unwrap_or(0),
            BrowserStorageArea::Session => {
                let removed = self.session_storage.len();
                self.session_storage.clear();
                removed
            }
        }
    }

    fn run_control_event(&mut self, control_id: usize, event: BrowserScriptEvent) -> bool {
        let before = self.script_stats.mutations;
        if let Some(node_id) = self
            .controls
            .get(control_id)
            .and_then(|control| control.dom_node)
        {
            self.run_event_handlers(node_id, event);
        }
        if event == BrowserScriptEvent::Submit {
            if let Some(form_node) = self
                .controls
                .get(control_id)
                .and_then(|control| control.form_id)
                .and_then(|form_id| self.forms.get(form_id))
                .and_then(|form| form.dom_node)
            {
                self.run_event_handlers(form_node, event);
            }
        }
        if self.script_stats.mutations != before {
            self.sync_source_from_dom();
            true
        } else {
            false
        }
    }

    fn run_event_handlers(&mut self, node_id: usize, event: BrowserScriptEvent) {
        let handlers: Vec<String> = self
            .script_handlers
            .iter()
            .filter(|handler| handler.node_id == node_id && handler.event == event)
            .map(|handler| handler.code.clone())
            .collect();
        for code in handlers {
            self.execute_script(&code, 1);
        }
    }

    fn sync_source_from_dom(&mut self) {
        self.source = self.dom.to_html();
    }
}

impl BrowserDomDocument {
    fn new() -> Self {
        Self {
            nodes: vec![BrowserDomNode {
                parent: None,
                children: Vec::new(),
                kind: BrowserDomNodeKind::Element {
                    name: String::from("document"),
                    attrs: Vec::new(),
                },
            }],
            root: 0,
        }
    }

    fn push_element(&mut self, parent: usize, name: &str, attrs: Vec<BrowserDomAttr>) -> usize {
        self.push_node(
            parent,
            BrowserDomNodeKind::Element {
                name: String::from(name),
                attrs,
            },
        )
    }

    fn push_text(&mut self, parent: usize, text: String) {
        if clean_inline_text(&text).is_empty() {
            return;
        }
        self.push_node(parent, BrowserDomNodeKind::Text(text));
    }

    fn push_text_raw(&mut self, parent: usize, text: String) {
        self.push_node(parent, BrowserDomNodeKind::Text(text));
    }

    fn push_node(&mut self, parent: usize, kind: BrowserDomNodeKind) -> usize {
        if self.nodes.len() >= MAX_DOM_NODES {
            return parent;
        }
        let idx = self.nodes.len();
        self.nodes.push(BrowserDomNode {
            parent: Some(parent),
            children: Vec::new(),
            kind,
        });
        if let Some(parent) = self.nodes.get_mut(parent) {
            parent.children.push(idx);
        }
        idx
    }

    fn to_html(&self) -> String {
        let mut out = String::new();
        if let Some(root) = self.nodes.get(self.root) {
            for child in &root.children {
                self.push_node_html(*child, &mut out);
            }
        }
        out
    }

    fn push_node_html(&self, node_id: usize, out: &mut String) {
        let Some(node) = self.nodes.get(node_id) else {
            return;
        };
        match &node.kind {
            BrowserDomNodeKind::Text(text) => push_html_text_escaped(out, text),
            BrowserDomNodeKind::Element { name, attrs } => {
                out.push('<');
                out.push_str(name);
                for attr in attrs.iter().take(MAX_DOM_ATTRS) {
                    out.push(' ');
                    out.push_str(&attr.name);
                    if !attr.value.is_empty() {
                        out.push_str("=\"");
                        push_html_attr_escaped(out, &attr.value);
                        out.push('"');
                    }
                }
                out.push('>');
                if is_void_element(name) {
                    return;
                }
                for child in &node.children {
                    self.push_node_html(*child, out);
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
        }
    }
}

impl BrowserFormControlKind {
    fn accepts_text(self) -> bool {
        matches!(self, Self::Text | Self::TextArea)
    }

    fn can_submit(self) -> bool {
        matches!(self, Self::Submit | Self::Image)
    }
}

impl BrowserFormControlState {
    fn is_focusable(&self) -> bool {
        !self.disabled
            && !matches!(
                self.kind,
                BrowserFormControlKind::Hidden | BrowserFormControlKind::Image
            )
    }

    fn successful(&self, is_submitter: bool) -> bool {
        if self.disabled || self.name.is_empty() {
            return false;
        }
        match self.kind {
            BrowserFormControlKind::Hidden
            | BrowserFormControlKind::Text
            | BrowserFormControlKind::TextArea
            | BrowserFormControlKind::Select => true,
            BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio => self.checked,
            BrowserFormControlKind::Submit | BrowserFormControlKind::Image => is_submitter,
            BrowserFormControlKind::Button | BrowserFormControlKind::Reset => false,
        }
    }

    fn submit_value(&self) -> String {
        if self.kind == BrowserFormControlKind::Checkbox
            || self.kind == BrowserFormControlKind::Radio
        {
            if self.value.is_empty() {
                String::from("on")
            } else {
                self.value.clone()
            }
        } else {
            self.value.clone()
        }
    }

    fn reset_to_default(&mut self) {
        match self.kind {
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => {
                self.value = self.default_value.clone();
            }
            BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio => {
                self.checked = self.default_checked;
            }
            BrowserFormControlKind::Select => {
                self.selected = self
                    .default_selected
                    .min(self.options.len().saturating_sub(1));
                if let Some(option) = self.options.first() {
                    self.value = self
                        .options
                        .get(self.selected)
                        .unwrap_or(option)
                        .value
                        .clone();
                }
            }
            _ => {}
        }
    }
}

enum BrowserControlActivation {
    Ignored,
    Focused,
    Changed,
    Navigate(String),
    Post { url: String, body: String },
    DomEvent(String),
}

enum BrowserScriptTarget {
    Id(String),
    Selector(String),
    SelectorAll(String, usize),
}

#[derive(Clone)]
enum BrowserScriptProperty {
    TextContent,
    ClassName,
    Value,
    Checked,
    Disabled,
    Style(String),
}

#[derive(Clone, Copy)]
enum BrowserStorageArea {
    Local,
    Session,
}

#[derive(Clone, Copy)]
enum BrowserStorageMethod {
    SetItem,
    RemoveItem,
    Clear,
}

struct BrowserStorageCall {
    area: BrowserStorageArea,
    method: BrowserStorageMethod,
    key: Option<String>,
    value_expr: Option<String>,
}

#[derive(Clone, Copy)]
enum ClassListOp {
    Add,
    Remove,
    Toggle,
}

struct ClassListCall {
    target: BrowserScriptTarget,
    op: ClassListOp,
    class_name: String,
}

#[derive(Clone, Copy)]
enum BrowserAttributeOp {
    Set,
    Remove,
}

struct BrowserAttributeCall {
    target: BrowserScriptTarget,
    op: BrowserAttributeOp,
    name: String,
    value_expr: Option<String>,
}

#[derive(Clone, Copy)]
enum BrowserFetchMethod {
    Get,
    Post,
}

struct BrowserFetchRequest {
    url: String,
    method: BrowserFetchMethod,
    body: String,
}

struct PendingOption {
    control_id: usize,
    value: Option<String>,
    label: String,
    selected: bool,
}

fn scan_dom_and_controls(body: &str, base_url: &str, document: &mut BrowserDocumentState) {
    let mut stack = vec![document.dom.root];
    let mut names = vec![String::from("document")];
    let mut form_stack: Vec<usize> = Vec::new();
    let mut text = String::new();
    let mut active_textarea: Option<usize> = None;
    let mut active_button: Option<(usize, String)> = None;
    let mut active_select: Option<usize> = None;
    let mut pending_option: Option<PendingOption> = None;
    let bytes = body.as_bytes();
    let lower_body = lowercase_ascii(body);
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if body[i..].starts_with("<!--") {
                if let Some(end_rel) = body[i + 4..].find("-->") {
                    i += end_rel + 7;
                } else {
                    i += 4;
                }
                continue;
            }
            flush_dom_text(document, &stack, &mut text);
            if let Some(end_rel) = find_tag_end(&body[i..]) {
                let tag = body[i + 1..i + end_rel].trim();
                let lower_tag = lowercase_ascii(tag);
                let name = tag_name_of(&lower_tag);
                let closing = lower_tag.starts_with('/');
                if closing {
                    match name {
                        "form" => {
                            form_stack.pop();
                        }
                        "textarea" => {
                            if let Some(control_id) = active_textarea {
                                finalize_textarea_value(document, control_id);
                            }
                            active_textarea = None;
                        }
                        "button" => {
                            finalize_button_text(document, &mut active_button);
                        }
                        "select" => {
                            active_select = None;
                        }
                        "option" => {
                            finish_pending_option(document, &mut pending_option);
                        }
                        _ => {}
                    }
                    pop_dom_stack(&mut stack, &mut names, name);
                    i += end_rel + 1;
                    continue;
                }

                if is_raw_text_suppressed_element(name) {
                    i = skip_raw_text_element(body, &lower_body, i + end_rel + 1, name);
                    continue;
                }

                repair_dom_before_start(document, &mut stack, &mut names, name);
                let parent = *stack.last().unwrap_or(&document.dom.root);
                let attrs = parse_dom_attrs(tag);
                let node = document.dom.push_element(parent, name, attrs);
                let self_closing = lower_tag.ends_with('/') || is_void_element(name);
                if !self_closing {
                    stack.push(node);
                    names.push(String::from(name));
                }

                match name {
                    "form" => {
                        if document.forms.len() < MAX_FORM_CONTROLS {
                            let id = document.forms.len();
                            document.forms.push(BrowserFormState {
                                action: form_action_url_any(tag, base_url),
                                method: form_method_for_tag(tag),
                                dom_node: Some(node),
                            });
                            form_stack.push(id);
                        }
                    }
                    "input" => {
                        push_document_input_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        );
                    }
                    "button" => {
                        active_button = push_document_button_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        )
                        .map(|control_id| (control_id, String::new()));
                    }
                    "select" => {
                        active_select = push_document_select_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        );
                    }
                    "textarea" => {
                        active_textarea = push_document_textarea_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        );
                    }
                    "option" => {
                        finish_pending_option(document, &mut pending_option);
                        if let Some(control_id) = active_select {
                            pending_option = Some(PendingOption {
                                control_id,
                                value: attr_value(tag, "value")
                                    .map(|value| clean_inline_text(&decode_entities(&value))),
                                label: String::new(),
                                selected: has_attr(tag, "selected"),
                            });
                        }
                    }
                    _ => {}
                }
                i += end_rel + 1;
                continue;
            }
        }
        let c = bytes[i] as char;
        text.push(c);
        if let Some(control_id) = active_textarea {
            if let Some(control) = document.controls.get_mut(control_id) {
                if control.value.len() < MAX_FORM_VALUE {
                    control.value.push(c);
                    control.default_value.push(c);
                }
            }
        }
        if let Some((_, text)) = active_button.as_mut() {
            if text.len() < MAX_FORM_VALUE {
                text.push(c);
            }
        }
        if let Some(option) = pending_option.as_mut() {
            option.label.push(c);
        }
        i += 1;
    }
    flush_dom_text(document, &stack, &mut text);
    finalize_button_text(document, &mut active_button);
    finish_pending_option(document, &mut pending_option);
}

fn flush_dom_text(document: &mut BrowserDocumentState, stack: &[usize], text: &mut String) {
    if text.is_empty() {
        return;
    }
    let parent = *stack.last().unwrap_or(&document.dom.root);
    document.dom.push_text(parent, decode_entities(text));
    text.clear();
}

fn repair_dom_before_start(
    document: &mut BrowserDocumentState,
    stack: &mut Vec<usize>,
    names: &mut Vec<String>,
    name: &str,
) {
    if matches!(name, "td" | "th") {
        while names
            .last()
            .map(|open| open == "td" || open == "th")
            .unwrap_or(false)
        {
            stack.pop();
            names.pop();
        }
    }
    if name == "tr" {
        while names
            .last()
            .map(|open| open == "td" || open == "th")
            .unwrap_or(false)
        {
            stack.pop();
            names.pop();
        }
        while names.last().map(|open| open == "tr").unwrap_or(false) {
            stack.pop();
            names.pop();
        }
    }
    if name == "li" && names.iter().any(|open| open == "li") {
        pop_dom_stack(stack, names, "li");
    }
    if is_block_boundary(name) && names.iter().any(|open| open == "p") {
        pop_dom_stack(stack, names, "p");
    }
    if stack.is_empty() {
        stack.push(document.dom.root);
        names.push(String::from("document"));
    }
}

fn pop_dom_stack(stack: &mut Vec<usize>, names: &mut Vec<String>, name: &str) {
    if stack.len() <= 1 {
        return;
    }
    if let Some(pos) = names.iter().rposition(|entry| entry == name) {
        let keep = pos.max(1);
        stack.truncate(keep);
        names.truncate(keep);
    }
}

fn parse_dom_attrs(tag: &str) -> Vec<BrowserDomAttr> {
    let mut out = Vec::new();
    let bytes = tag.as_bytes();
    let mut pos = tag_name_of(&lowercase_ascii(tag)).len();
    while pos < bytes.len() && out.len() < MAX_DOM_ATTRS {
        while pos < bytes.len()
            && (bytes[pos].is_ascii_whitespace() || matches!(bytes[pos], b'/' | b'<'))
        {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
        {
            pos += 1;
        }
        if start == pos {
            pos = pos.saturating_add(1);
            continue;
        }
        let name = lowercase_ascii(&tag[start..pos]);
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut value = String::new();
        if bytes.get(pos) == Some(&b'=') {
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
                let quote = bytes[pos];
                pos += 1;
                let value_start = pos;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                value = decode_entities(&tag[value_start..pos]);
                pos = pos.saturating_add(1);
            } else {
                let value_start = pos;
                while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                    pos += 1;
                }
                value = decode_entities(&tag[value_start..pos]);
            }
        }
        out.push(BrowserDomAttr { name, value });
    }
    out
}

fn form_method_for_tag(tag: &str) -> BrowserFormMethod {
    attr_value(tag, "method")
        .map(|method| {
            if method.eq_ignore_ascii_case("post") {
                BrowserFormMethod::Post
            } else {
                BrowserFormMethod::Get
            }
        })
        .unwrap_or(BrowserFormMethod::Get)
}

fn form_action_url_any(tag: &str, base_url: &str) -> String {
    let action = attr_value(tag, "action").unwrap_or_else(|| String::from(base_url));
    if action.trim().is_empty() {
        String::from(base_url)
    } else {
        resolve_url(base_url, &action)
    }
}

fn push_document_input_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
    dom_node: Option<usize>,
) -> Option<usize> {
    let input_type = lowercase_ascii(
        attr_value(tag, "type")
            .unwrap_or_else(|| String::from("text"))
            .trim(),
    );
    let kind = match input_type.as_str() {
        "hidden" => BrowserFormControlKind::Hidden,
        "checkbox" => BrowserFormControlKind::Checkbox,
        "radio" => BrowserFormControlKind::Radio,
        "submit" => BrowserFormControlKind::Submit,
        "button" => BrowserFormControlKind::Button,
        "reset" => BrowserFormControlKind::Reset,
        "image" => BrowserFormControlKind::Image,
        _ => BrowserFormControlKind::Text,
    };
    let label = if kind == BrowserFormControlKind::Text {
        input_field_label(tag, &input_type)
    } else {
        form_control_label(tag, &input_type)
    };
    let value = input_value(tag);
    let checked = has_attr(tag, "checked");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
            kind,
            name: control_name(tag),
            label,
            value: value.clone(),
            default_value: value,
            checked,
            default_checked: checked,
            disabled: has_attr(tag, "disabled"),
            chars: input_size_chars(tag),
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_button_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
    dom_node: Option<usize>,
) -> Option<usize> {
    let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
    let kind = if button_type.eq_ignore_ascii_case("button") {
        BrowserFormControlKind::Button
    } else if button_type.eq_ignore_ascii_case("reset") {
        BrowserFormControlKind::Reset
    } else {
        BrowserFormControlKind::Submit
    };
    let label = form_control_label(tag, "button");
    let value = attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(|| label.clone());
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
            kind,
            name: control_name(tag),
            label,
            value: value.clone(),
            default_value: value,
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: 12,
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_select_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
    dom_node: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "select");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
            kind: BrowserFormControlKind::Select,
            name: control_name(tag),
            label,
            value: String::new(),
            default_value: String::new(),
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: 20,
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_textarea_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
    dom_node: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "textarea");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
            kind: BrowserFormControlKind::TextArea,
            name: control_name(tag),
            label,
            value: String::new(),
            default_value: String::new(),
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: input_size_chars(tag),
            rows: attr_value(tag, "rows")
                .and_then(|value| parse_dimension(&value))
                .unwrap_or(3)
                .clamp(2, 8),
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_control(
    document: &mut BrowserDocumentState,
    control: BrowserFormControlState,
) -> Option<usize> {
    if document.controls.len() >= MAX_FORM_CONTROLS {
        return None;
    }
    let id = document.controls.len();
    document.controls.push(control);
    Some(id)
}

fn finish_pending_option(document: &mut BrowserDocumentState, pending: &mut Option<PendingOption>) {
    let Some(option) = pending.take() else {
        return;
    };
    let Some(control) = document.controls.get_mut(option.control_id) else {
        return;
    };
    if control.options.len() >= MAX_FORM_OPTIONS {
        return;
    }
    let label = clean_inline_text(&decode_entities(&option.label));
    let label = if label.is_empty() {
        option
            .value
            .clone()
            .unwrap_or_else(|| String::from("option"))
    } else {
        label
    };
    let value = option.value.unwrap_or_else(|| label.clone());
    if option.selected {
        control.selected = control.options.len();
        control.default_selected = control.selected;
    }
    control.options.push(BrowserSelectOption { label, value });
}

fn finalize_textarea_value(document: &mut BrowserDocumentState, control_id: usize) {
    let Some(control) = document.controls.get_mut(control_id) else {
        return;
    };
    if control.kind != BrowserFormControlKind::TextArea {
        return;
    }
    control.value = decode_entities(&control.value);
    control.default_value = control.value.clone();
    if control.value.len() > MAX_FORM_VALUE {
        let mut trimmed = String::new();
        for c in control.value.chars() {
            if trimmed.len().saturating_add(c.len_utf8()) > MAX_FORM_VALUE {
                break;
            }
            trimmed.push(c);
        }
        control.value = trimmed;
        control.default_value = control.value.clone();
    }
}

fn finalize_button_text(
    document: &mut BrowserDocumentState,
    active_button: &mut Option<(usize, String)>,
) {
    let Some((control_id, text)) = active_button.take() else {
        return;
    };
    let label = clean_inline_text(&decode_entities(&text));
    if label.is_empty() {
        return;
    }
    let Some(control) = document.controls.get_mut(control_id) else {
        return;
    };
    let fallback_value =
        control.default_value == control.label || control.default_value == "button";
    control.label = label.clone();
    if fallback_value {
        control.value = label.clone();
        control.default_value = label;
    }
}

fn control_name(tag: &str) -> String {
    attr_value(tag, "name")
        .map(|name| clean_inline_text(&decode_entities(&name)))
        .unwrap_or_else(String::new)
}

fn is_inline_tag(name: &str) -> bool {
    matches!(
        name,
        "span"
            | "strong"
            | "em"
            | "b"
            | "i"
            | "small"
            | "big"
            | "sub"
            | "sup"
            | "s"
            | "u"
            | "del"
            | "ins"
            | "mark"
            | "cite"
            | "abbr"
            | "time"
            | "var"
            | "samp"
            | "kbd"
            | "wbr"
            | "bdi"
            | "bdo"
            | "data"
            | "q"
            | "dfn"
            | "label"
            | "output"
            | "meter"
            | "progress"
            | "nobr"
            | "font"
            | "tt"
            | "acronym"
            | "strike"
            | "blink"
            | "marquee"
    )
}

fn tag_alignment(tag: &str, name: &str) -> Option<BrowserAlign> {
    if name == "center" {
        return Some(BrowserAlign::Center);
    }
    if let Some(value) = attr_value(tag, "align") {
        return parse_alignment(&value);
    }
    if let Some(style) = attr_value(tag, "style") {
        let style = lowercase_ascii(&style);
        if style.contains("text-align:center") || style.contains("text-align: center") {
            return Some(BrowserAlign::Center);
        }
        if style.contains("text-align:right") || style.contains("text-align: right") {
            return Some(BrowserAlign::Right);
        }
        if style.contains("text-align:left") || style.contains("text-align: left") {
            return Some(BrowserAlign::Left);
        }
        if style.contains("margin:") && style.contains("auto") {
            return Some(BrowserAlign::Center);
        }
    }
    None
}

fn parse_alignment(value: &str) -> Option<BrowserAlign> {
    let value = lowercase_ascii(value.trim());
    match value.as_str() {
        "center" | "middle" => Some(BrowserAlign::Center),
        "right" => Some(BrowserAlign::Right),
        "left" => Some(BrowserAlign::Left),
        _ => None,
    }
}

fn repair_html_before_start(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
    name: &str,
) {
    if matches!(name, "td" | "th") && state.in_table_cell {
        finish_table_cell(state);
    }
    if name == "tr" {
        if state.in_table_cell {
            finish_table_cell(state);
        }
        if state.in_table && !state.table_row.is_empty() {
            finish_table_row(out, state, cols);
        }
    }
    if name == "li" && state.has_open_element("li") {
        close_implicit_element(out, text, cols, state, "li");
    }
    if is_block_boundary(name) && state.has_open_element("p") {
        close_implicit_element(out, text, cols, state, "p");
    }
}

fn close_implicit_element(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
    name: &str,
) {
    flush_flow_text(out, text, cols, state);
    state.pop_align(name);
    state.pop_style(name);
    state.pop_list_style(name);
    state.pop_pre_style(name);
    state.pop_suppressed_text(name);
    state.pop_open_element(name);
    if matches!(name, "p" | "li") {
        push_blank_line(out);
    }
}

fn is_block_boundary(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}

fn handle_tag(
    tag: &str,
    lower_tag: &str,
    styles: &StyleHints,
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
    base_url: &str,
    cols: usize,
) {
    let tag = tag.trim();
    let name = tag_name_of(lower_tag);
    let closing = lower_tag.starts_with('/');
    let tag_style = styles.computed_for_tag(lower_tag, name);
    let mut controls = controls;
    if closing {
        state.pop_suppressed_text(name);
        state.pop_open_element(name);
    }

    if state.in_table_cell {
        handle_table_cell_tag(
            tag,
            out,
            state,
            controls.as_deref_mut(),
            base_url,
            cols,
            name,
            closing,
        );
        return;
    }

    if is_inline_tag(name) {
        let has_inline_effect = tag_style.align.is_some()
            || !tag_style.line.is_default()
            || tag_style.preformatted
            || tag_style.display == CssDisplay::Block;
        if !has_inline_effect {
            return;
        }
        flush_flow_text(out, text, cols, state);
        if closing {
            state.pop_align(name);
            state.pop_style(name);
            state.pop_list_style(name);
            state.pop_pre_style(name);
        } else if !is_void_element(name) {
            state.push_open_element(name);
            if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
                state.push_align(name, align);
            }
            state.push_style(name, tag_style.line);
            if let Some(list_style) = tag_style.list_style {
                state.push_list_style(name, list_style);
            }
            if tag_style.preformatted {
                state.push_pre_style(name);
            }
        }
        return;
    }

    flush_flow_text(out, text, cols, state);
    if closing {
        state.pop_align(name);
        state.pop_style(name);
        state.pop_list_style(name);
        state.pop_pre_style(name);
    } else if !is_void_element(name) {
        state.push_open_element(name);
        if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
            state.push_align(name, align);
        }
    }
    if !closing && !is_void_element(name) {
        state.push_style(name, tag_style.line);
        if let Some(list_style) = tag_style.list_style {
            state.push_list_style(name, list_style);
        }
        if tag_style.preformatted {
            state.push_pre_style(name);
        }
    }

    match name {
        "a" => {
            if closing {
                state.link = None;
            } else if let Some(href) = attr_value(tag, "href") {
                state.link = Some(resolve_url(base_url, &decode_entities(&href)));
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if closing {
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.kind = BrowserLineKind::Heading;
            }
        }
        "pre" => {
            if closing {
                state.preformatted = false;
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.preformatted = true;
                state.kind = BrowserLineKind::Code;
            }
        }
        "code" => {
            state.kind = if closing {
                BrowserLineKind::Text
            } else {
                BrowserLineKind::Code
            };
        }
        "blockquote" => {
            if closing {
                state.quote_depth = state.quote_depth.saturating_sub(1);
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.quote_depth = state.quote_depth.saturating_add(1);
                state.kind = BrowserLineKind::Quote;
            }
        }
        "ul" => {
            if closing {
                state.list_depth = state.list_depth.saturating_sub(1);
            } else {
                state.list_depth = state.list_depth.saturating_add(1);
            }
            push_blank_line(out);
        }
        "ol" => {
            if closing {
                state.ordered_stack.pop();
                state.list_depth = state.list_depth.saturating_sub(1);
            } else {
                state.ordered_stack.push(1);
                state.list_depth = state.list_depth.saturating_add(1);
            }
            push_blank_line(out);
        }
        "li" => {
            push_blank_line(out);
            state.pending_prefix = Some(list_prefix(state));
        }
        "form" => {
            if closing {
                state.form_action = None;
                state.form_fields.clear();
                push_blank_line(out);
            } else {
                state.form_action = form_action_url(tag, base_url);
                state.form_fields.clear();
                push_blank_line(out);
            }
        }
        "input" => {
            if !closing {
                push_input_line(out, tag, state, controls.as_deref_mut(), tag_style);
            }
        }
        "button" => {
            if !closing {
                let interactive = controls.is_some();
                push_button_line(out, tag, state, controls.as_deref_mut());
                if interactive {
                    state.push_suppressed_text(name);
                }
            }
        }
        "select" => {
            if !closing {
                push_named_control_line(out, "select", tag, state, controls.as_deref_mut());
                state.push_suppressed_text(name);
            }
        }
        "textarea" => {
            if !closing {
                push_named_control_line(out, "textarea", tag, state, controls.as_deref_mut());
                state.push_suppressed_text(name);
            }
        }
        "img" => push_image_line(out, tag, base_url, state, tag_style),
        "table" => {
            if closing {
                finish_table_row(out, state, cols);
                state.in_table = false;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.in_table = true;
                state.table_row.clear();
            }
        }
        "tr" => {
            if closing {
                finish_table_row(out, state, cols);
            } else {
                state.in_table = true;
                state.table_row.clear();
            }
        }
        "td" | "th" => {
            state.in_table = true;
            state.in_table_cell = true;
            state.table_cell_is_header = name == "th";
            state.table_cell_align =
                tag_alignment(tag, name).unwrap_or_else(|| state.current_align());
            state.table_cell_text.clear();
        }
        "thead" | "tbody" | "tfoot" | "colgroup" | "col" | "caption" => {}
        "br" => push_blank_line(out),
        "hr" => out.push(kind_line(&rule_line(cols), BrowserLineKind::Muted)),
        "p" | "div" | "section" | "article" | "main" | "aside" | "header" | "footer" | "nav"
        | "figure" | "figcaption" | "address" | "dl" | "dt" | "dd" | "center" => {
            push_blank_line(out);
        }
        _ => {}
    }
}

fn form_action_url(tag: &str, base_url: &str) -> Option<String> {
    let method = attr_value(tag, "method").unwrap_or_else(|| String::from("get"));
    if method.eq_ignore_ascii_case("post") {
        return None;
    }
    let action = attr_value(tag, "action").unwrap_or_else(|| String::from(base_url));
    if action.trim().is_empty() {
        Some(String::from(base_url))
    } else {
        Some(resolve_url(base_url, &action))
    }
}

fn record_input_field(state: &mut HtmlRenderState, tag: &str, input_type: &str) {
    if state.form_action.is_none() || has_attr(tag, "disabled") || state.form_fields.len() >= 64 {
        return;
    }
    if matches!(input_type, "submit" | "button" | "reset" | "image" | "file") {
        return;
    }
    if matches!(input_type, "checkbox" | "radio") && !has_attr(tag, "checked") {
        return;
    }
    let Some(name) = attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
    else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let value = if matches!(input_type, "checkbox" | "radio") {
        attr_value(tag, "value")
            .map(|value| clean_inline_text(&decode_entities(&value)))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| String::from("on"))
    } else {
        input_value(tag)
    };
    state.form_fields.push(FormField { name, value });
}

fn record_named_form_field(state: &mut HtmlRenderState, tag: &str) {
    if state.form_action.is_none() || has_attr(tag, "disabled") || state.form_fields.len() >= 64 {
        return;
    }
    let Some(name) = attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
    else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let value = attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(String::new);
    state.form_fields.push(FormField { name, value });
}

fn form_submit_url(state: &HtmlRenderState, submit_tag: Option<&str>) -> Option<String> {
    let mut out = state.form_action.clone()?;
    let mut wrote = out.contains('?');
    for field in &state.form_fields {
        append_query_param(&mut out, &mut wrote, &field.name, &field.value);
    }
    if let Some(tag) = submit_tag {
        if !has_attr(tag, "disabled") {
            if let Some(name) =
                attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
            {
                if !name.is_empty() {
                    let value = attr_value(tag, "value")
                        .map(|value| clean_inline_text(&decode_entities(&value)))
                        .unwrap_or_else(String::new);
                    append_query_param(&mut out, &mut wrote, &name, &value);
                }
            }
        }
    }
    Some(out)
}

fn append_query_param(out: &mut String, wrote: &mut bool, name: &str, value: &str) {
    out.push(if *wrote { '&' } else { '?' });
    push_query_encoded(out, name);
    out.push('=');
    push_query_encoded(out, value);
    *wrote = true;
}

fn push_input_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
    tag_style: TagStyle,
) {
    let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
    let input_type = lowercase_ascii(input_type.trim());
    let interactive = controls.and_then(|controls| controls.next());
    if input_type == "hidden" {
        if interactive.is_none() {
            record_input_field(state, tag, &input_type);
        }
        return;
    }
    let mut text = String::new();
    let mut link = None;
    let control;
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    match input_type.as_str() {
        "submit" | "button" | "reset" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, &input_type));
            text.push_str("[button] ");
            text.push_str(&label);
            if live.is_none() {
                link = if input_type == "submit" {
                    form_submit_url(state, Some(tag))
                } else {
                    Some(browser_event_url(&label))
                };
            }
            control = BrowserControl::Button { label };
        }
        "checkbox" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[checkbox] ");
            text.push_str(&label);
            let checked = live
                .map(|control| control.checked)
                .unwrap_or_else(|| has_attr(tag, "checked"));
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::Checkbox { label, checked };
        }
        "radio" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[radio] ");
            text.push_str(&label);
            let checked = live
                .map(|control| control.checked)
                .unwrap_or_else(|| has_attr(tag, "checked"));
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::Radio { label, checked };
        }
        "search" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[search] ");
            text.push_str(&label);
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::TextInput {
                label: live
                    .map(|control| control.label.clone())
                    .unwrap_or_else(|| input_control_label(tag, &label)),
                value: live
                    .map(|control| control.value.clone())
                    .unwrap_or_else(|| input_value(tag)),
                chars: live
                    .map(|control| control.chars)
                    .unwrap_or_else(|| input_size_chars(tag)),
            };
        }
        "image" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, &input_type));
            text.push_str("[image button] ");
            text.push_str(&label);
            link = if live.is_none() {
                form_submit_url(state, Some(tag))
            } else {
                None
            };
            control = BrowserControl::Button { label };
        }
        _ => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[input] ");
            text.push_str(&label);
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::TextInput {
                label: live
                    .map(|control| control.label.clone())
                    .unwrap_or_else(|| input_control_label(tag, &label)),
                value: live
                    .map(|control| control.value.clone())
                    .unwrap_or_else(|| input_value(tag)),
                chars: live
                    .map(|control| control.chars)
                    .unwrap_or_else(|| input_size_chars(tag)),
            };
        }
    }
    out.push(
        BrowserLine::new(
            text,
            link.clone(),
            line_kind_for_link(&link, BrowserLineKind::Code),
        )
        .aligned(state.current_align())
        .styled(state.current_line_style().merged(tag_style.line))
        .with_control(control)
        .with_control_id(control_id),
    );
}

fn push_button_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    state: &HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
) {
    let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
    let interactive = controls.and_then(|controls| controls.next());
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    let label = live
        .map(|control| control.label.clone())
        .unwrap_or_else(|| form_control_label(tag, "button"));
    let link = if live.is_some() {
        None
    } else if button_type.eq_ignore_ascii_case("submit") {
        form_submit_url(state, Some(tag))
    } else {
        Some(browser_event_url(&label))
    };
    let has_form_value = attr_value(tag, "value").is_some()
        || attr_value(tag, "name").is_some()
        || attr_value(tag, "id").is_some();
    if link.is_none() && control_id.is_none() && !has_form_value {
        return;
    }
    out.push(
        BrowserLine::new(
            format!("[button] {}", label),
            link.clone(),
            line_kind_for_link(&link, BrowserLineKind::Code),
        )
        .aligned(state.current_align())
        .styled(state.current_line_style())
        .with_control(BrowserControl::Button { label })
        .with_control_id(control_id),
    );
}

fn push_named_control_line(
    out: &mut Vec<BrowserLine>,
    control: &str,
    tag: &str,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
) {
    let interactive = controls.and_then(|controls| controls.next());
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    let label = live
        .map(|control| control.label.clone())
        .unwrap_or_else(|| form_control_label(tag, control));
    let visual = if control == "textarea" {
        BrowserControl::TextArea {
            label: label.clone(),
            value: live
                .map(|control| control.value.clone())
                .unwrap_or_else(String::new),
            rows: live.map(|control| control.rows).unwrap_or_else(|| {
                attr_value(tag, "rows")
                    .and_then(|value| parse_dimension(&value))
                    .unwrap_or(3)
                    .clamp(2, 8)
            }),
        }
    } else {
        BrowserControl::Select {
            label: label.clone(),
            value: live.map(select_display_value).unwrap_or_else(String::new),
            options: live.map(|control| control.options.len()).unwrap_or(0),
        }
    };
    if live.is_none() {
        record_named_form_field(state, tag);
    }
    out.push(
        BrowserLine::new(
            format!("[{}] {}", control, label),
            None,
            BrowserLineKind::Code,
        )
        .aligned(state.current_align())
        .styled(state.current_line_style())
        .with_control(visual)
        .with_control_id(control_id),
    );
}

fn select_display_value(control: &BrowserFormControlState) -> String {
    control
        .options
        .get(control.selected)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| control.value.clone())
}

fn form_control_label(tag: &str, fallback: &str) -> String {
    for attr in ["aria-label", "placeholder", "value", "title", "name", "id"] {
        if let Some(value) = attr_value(tag, attr) {
            let decoded = clean_inline_text(&decode_entities(&value));
            if !decoded.is_empty() {
                return decoded;
            }
        }
    }
    String::from(fallback)
}

// For text/search/email fields, prefer name/placeholder over aria-label
// so the field shows its identity, not the descriptive label shared with a button.
fn input_field_label(tag: &str, fallback: &str) -> String {
    for attr in ["placeholder", "name", "id", "aria-label", "title"] {
        if let Some(value) = attr_value(tag, attr) {
            let decoded = clean_inline_text(&decode_entities(&value));
            if !decoded.is_empty() {
                return decoded;
            }
        }
    }
    String::from(fallback)
}

fn input_control_label(tag: &str, _fallback: &str) -> String {
    attr_value(tag, "placeholder")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(String::new)
}

fn input_value(tag: &str) -> String {
    attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(String::new)
}

fn input_size_chars(tag: &str) -> usize {
    attr_value(tag, "size")
        .and_then(|value| parse_dimension(&value))
        .unwrap_or(28)
        .clamp(8, 72)
}

fn handle_table_cell_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    state: &mut HtmlRenderState,
    mut controls: Option<&mut BrowserRenderControls<'_>>,
    base_url: &str,
    cols: usize,
    name: &str,
    closing: bool,
) {
    match name {
        "td" | "th" if closing => finish_table_cell(state),
        "tr" if closing => {
            finish_table_cell(state);
            finish_table_row(out, state, cols);
        }
        "table" if closing => {
            finish_table_cell(state);
            finish_table_row(out, state, cols);
            state.in_table = false;
            push_blank_line(out);
        }
        "br" => {
            if !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
        }
        "img" => {
            let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
            let label = attr_value(tag, "alt")
                .map(|alt| decode_entities(&alt))
                .filter(|alt| !alt.trim().is_empty())
                .unwrap_or_else(|| src.unwrap_or_else(|| String::from("image")));
            if !state.table_cell_text.ends_with(' ') && !state.table_cell_text.is_empty() {
                state.table_cell_text.push(' ');
            }
            state.table_cell_text.push_str("[image");
            if let Some(size) = image_size_label(tag) {
                state.table_cell_text.push(' ');
                state.table_cell_text.push_str(&size);
            }
            state.table_cell_text.push(' ');
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "a" => {
            if !closing {
                if let Some(href) = attr_value(tag, "href") {
                    state.cell_link = Some(resolve_url(base_url, &decode_entities(&href)));
                }
            }
        }
        "input" if !closing => {
            let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
            let input_type = lowercase_ascii(input_type.trim());
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            if input_type == "hidden" {
                if live.is_none() {
                    record_input_field(state, tag, &input_type);
                }
                return;
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            match input_type.as_str() {
                "submit" | "button" | "reset" | "image" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| form_control_label(tag, &input_type));
                    if live.is_none() && state.cell_form_link.is_none() {
                        state.cell_form_link = if input_type == "submit" || input_type == "image" {
                            form_submit_url(state, Some(tag))
                        } else {
                            Some(browser_event_url(&label))
                        };
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Button {
                                label: label.clone(),
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[btn:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                "checkbox" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let checked = live
                        .map(|control| control.checked)
                        .unwrap_or_else(|| has_attr(tag, "checked"));
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Checkbox {
                                label: label.clone(),
                                checked,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[checkbox:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                "radio" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let checked = live
                        .map(|control| control.checked)
                        .unwrap_or_else(|| has_attr(tag, "checked"));
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Radio {
                                label: label.clone(),
                                checked,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[radio:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                _ => {
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let chars = live
                        .map(|control| control.chars)
                        .unwrap_or_else(|| input_size_chars(tag));
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::TextInput {
                                label: live
                                    .map(|control| control.label.clone())
                                    .unwrap_or_else(|| input_control_label(tag, &label)),
                                value: live
                                    .map(|control| control.value.clone())
                                    .unwrap_or_else(|| input_value(tag)),
                                chars,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[field:");
                    state.table_cell_text.push_str(&format!("{}", chars));
                    state.table_cell_text.push(':');
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
            }
        }
        "button" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "button"));
            if live.is_none() && state.cell_form_link.is_none() {
                let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
                state.cell_form_link = if button_type.eq_ignore_ascii_case("submit") {
                    form_submit_url(state, Some(tag))
                } else {
                    Some(browser_event_url(&label))
                };
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::Button {
                        label: label.clone(),
                    },
                ));
            }
            state.table_cell_text.push_str("[btn:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "select" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "select"));
            if live.is_none() {
                record_named_form_field(state, tag);
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::Select {
                        label: label.clone(),
                        value: live.map(select_display_value).unwrap_or_else(String::new),
                        options: live.map(|control| control.options.len()).unwrap_or(0),
                    },
                ));
            }
            state.table_cell_text.push_str("[select:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "textarea" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "textarea"));
            if live.is_none() {
                record_named_form_field(state, tag);
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::TextArea {
                        label: label.clone(),
                        value: live
                            .map(|control| control.value.clone())
                            .unwrap_or_else(String::new),
                        rows: live.map(|control| control.rows).unwrap_or(3),
                    },
                ));
            }
            state.table_cell_text.push_str("[textarea:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        _ => {}
    }
}

fn push_image_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    base_url: &str,
    state: &HtmlRenderState,
    tag_style: TagStyle,
) {
    let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
    let label = attr_value(tag, "alt")
        .map(|alt| decode_entities(&alt))
        .filter(|alt| !alt.trim().is_empty())
        .unwrap_or_else(|| src.clone().unwrap_or_else(|| String::from("image")));
    let hint = image_hint_for_tag(tag, tag_style);
    let mut text = String::from("[image");
    if let Some(size) = image_size_label_for_hint(hint) {
        text.push(' ');
        text.push_str(&size);
    }
    text.push_str("] ");
    text.push_str(&label);
    out.push(
        BrowserLine::new(text, src, BrowserLineKind::Image)
            .aligned(state.current_align())
            .styled(state.current_line_style().merged(tag_style.line))
            .with_image_hint(hint),
    );
}

fn image_size_label(tag: &str) -> Option<String> {
    image_size_label_for_hint(image_hint_for_tag(tag, TagStyle::default()))
}

fn image_hint_for_tag(tag: &str, tag_style: TagStyle) -> ImageHint {
    ImageHint {
        width: tag_style
            .width
            .or_else(|| attr_value(tag, "width").and_then(|value| parse_dimension(&value))),
        height: tag_style
            .height
            .or_else(|| attr_value(tag, "height").and_then(|value| parse_dimension(&value))),
    }
}

fn image_size_label_for_hint(hint: ImageHint) -> Option<String> {
    match (hint.width, hint.height) {
        (Some(width), Some(height)) => Some(format!("{}x{}", width, height)),
        (Some(width), None) => Some(format!("{}w", width)),
        (None, Some(height)) => Some(format!("{}h", height)),
        (None, None) => None,
    }
}

fn parse_dimension(value: &str) -> Option<usize> {
    let mut out = 0usize;
    let mut saw_digit = false;
    for b in value.trim().bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        out = out.saturating_mul(10).saturating_add((b - b'0') as usize);
    }
    if saw_digit && out > 0 && out <= 10_000 {
        Some(out)
    } else {
        None
    }
}

fn list_prefix(state: &mut HtmlRenderState) -> String {
    let mut out = String::new();
    for _ in 1..state.list_depth {
        out.push_str("  ");
    }
    match state.current_list_style() {
        CssListStyle::Decimal => {
            if let Some(next) = state.ordered_stack.last_mut() {
                out.push_str(&format!("{}. ", *next));
                *next = next.saturating_add(1);
            } else {
                out.push_str("1. ");
            }
        }
        CssListStyle::Circle => out.push_str("o "),
        CssListStyle::Square => out.push_str("- "),
        CssListStyle::None => {}
        CssListStyle::Disc => {
            if let Some(next) = state.ordered_stack.last_mut() {
                out.push_str(&format!("{}. ", *next));
                *next = next.saturating_add(1);
            } else {
                out.push_str("* ");
            }
        }
    }
    out
}

fn rule_line(cols: usize) -> String {
    let mut out = String::new();
    for _ in 0..cols.clamp(20, 80) {
        out.push('-');
    }
    out
}

fn push_form_cell_lines(out: &mut Vec<BrowserLine>, cell: TableCell) {
    let align = cell.align;
    let link = cell.link;
    let text = cell.text;
    let mut controls = cell.controls.into_iter();
    let mut i = 0usize;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'[' {
            if let Some(end) = text[i..].find(']') {
                let part = &text[i..i + end + 1];
                if let Some(label) = part.strip_prefix("[btn:").and_then(|s| s.strip_suffix(']')) {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Button {
                            label: String::from(label),
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[button] {}", label),
                            link.clone(),
                            line_kind_for_link(&link, BrowserLineKind::Code),
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[checkbox:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Checkbox {
                            label: String::from(label),
                            checked: false,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[checkbox] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[radio:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Radio {
                            label: String::from(label),
                            checked: false,
                        }
                    });
                    out.push(
                        BrowserLine::new(format!("[radio] {}", label), None, BrowserLineKind::Code)
                            .aligned(align)
                            .with_control(visual)
                            .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[select:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Select {
                            label: String::from(label),
                            value: String::new(),
                            options: 0,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[select] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[textarea:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::TextArea {
                            label: String::from(label),
                            value: String::new(),
                            rows: 3,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[textarea] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(rest) = part
                    .strip_prefix("[field:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let (chars, label) = rest
                        .split_once(':')
                        .map(|(chars, label)| {
                            (parse_dimension(chars).unwrap_or(28).clamp(8, 72), label)
                        })
                        .unwrap_or((28, rest));
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::TextInput {
                            label: String::from(label),
                            value: String::new(),
                            chars,
                        }
                    });
                    out.push(
                        BrowserLine::new(format!("[input] {}", label), None, BrowserLineKind::Code)
                            .aligned(align)
                            .with_control(visual)
                            .with_control_id(control_id),
                    );
                } else {
                    out.push(
                        BrowserLine::new(String::from(part), None, BrowserLineKind::Code)
                            .aligned(align),
                    );
                }
                i += end + 1;
            } else {
                break;
            }
        } else {
            let end = text[i..].find('[').map(|p| i + p).unwrap_or(text.len());
            let plain = text[i..end].trim();
            if !plain.is_empty() {
                out.push(
                    BrowserLine::new(String::from(plain), None, BrowserLineKind::Text)
                        .aligned(align),
                );
            }
            i = end;
        }
    }
}

fn finish_table_cell(state: &mut HtmlRenderState) {
    if !state.in_table_cell {
        return;
    }
    let text = clean_inline_text(&decode_entities(&state.table_cell_text));
    let link = if state.cell_has_form_control {
        state.cell_form_link.take()
    } else {
        state.cell_link.take()
    };
    state.table_row.push(TableCell {
        text,
        header: state.table_cell_is_header,
        link,
        is_form_row: state.cell_has_form_control,
        align: state.table_cell_align,
        controls: core::mem::take(&mut state.cell_controls),
    });
    state.table_cell_text.clear();
    state.table_cell_is_header = false;
    state.in_table_cell = false;
    state.cell_has_form_control = false;
    state.cell_form_link = None;
    state.cell_link = None;
    state.cell_controls.clear();
    state.pop_align("td");
    state.pop_align("th");
    state.pop_style("td");
    state.pop_style("th");
    state.pop_list_style("td");
    state.pop_list_style("th");
    state.pop_pre_style("td");
    state.pop_pre_style("th");
    state.pop_open_element("td");
    state.pop_open_element("th");
    state.table_cell_align = state.current_align();
}

fn finish_table_row(out: &mut Vec<BrowserLine>, state: &mut HtmlRenderState, cols: usize) {
    if state.table_row.is_empty() {
        return;
    }
    let has_form = state.table_row.iter().any(|c| c.is_form_row);
    if has_form {
        for cell in state.table_row.drain(..) {
            if cell.text.is_empty() {
                continue;
            }
            if cell.is_form_row {
                push_form_cell_lines(out, cell);
            } else {
                let kind = if cell.link.is_some() {
                    BrowserLineKind::Link
                } else {
                    BrowserLineKind::Text
                };
                out.push(BrowserLine::new(cell.text, cell.link, kind).aligned(cell.align));
            }
        }
        return;
    }
    let header = state.table_row.iter().any(|cell| cell.header);
    let row = format_table_row(&state.table_row, cols);
    out.push(kind_line(
        &row,
        if header {
            BrowserLineKind::Heading
        } else {
            BrowserLineKind::Code
        },
    ));
    if header {
        out.push(kind_line(
            &format_table_separator(&state.table_row, cols),
            BrowserLineKind::Muted,
        ));
    }
    state.table_row.clear();
}

fn format_table_row(cells: &[TableCell], cols: usize) -> String {
    let widths = table_column_widths(cells, cols);
    let mut out = String::from("|");
    for (idx, cell) in cells.iter().enumerate() {
        out.push(' ');
        push_truncated_padded(
            &mut out,
            &cell.text,
            widths
                .get(idx)
                .copied()
                .unwrap_or_else(|| table_cell_width(cells.len(), cols)),
        );
        out.push(' ');
        out.push('|');
    }
    out
}

fn format_table_separator(cells: &[TableCell], cols: usize) -> String {
    let widths = table_column_widths(cells, cols);
    let mut out = String::from("+");
    for idx in 0..cells.len() {
        let width = widths
            .get(idx)
            .copied()
            .unwrap_or_else(|| table_cell_width(cells.len(), cols));
        for _ in 0..(width + 2) {
            out.push('-');
        }
        out.push('+');
    }
    out
}

fn table_column_widths(cells: &[TableCell], cols: usize) -> Vec<usize> {
    if cells.is_empty() {
        return Vec::new();
    }
    let chrome = cells.len().saturating_mul(3).saturating_add(1);
    let budget = cols
        .saturating_sub(chrome)
        .max(cells.len().saturating_mul(8));
    let mut widths = Vec::new();
    let mut used = 0usize;
    for cell in cells {
        let wanted = clean_inline_text(&cell.text).chars().count().clamp(8, 32);
        widths.push(wanted);
        used = used.saturating_add(wanted);
    }
    while used > budget {
        let Some((idx, width)) = widths
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, width)| (idx, *width))
        else {
            break;
        };
        if width <= 8 {
            break;
        }
        widths[idx] = widths[idx].saturating_sub(1);
        used = used.saturating_sub(1);
    }
    while used < budget {
        let Some((idx, width)) = widths
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, width)| (idx, *width))
        else {
            break;
        };
        if width >= 32 {
            break;
        }
        widths[idx] = widths[idx].saturating_add(1);
        used = used.saturating_add(1);
    }
    widths
}

fn table_cell_width(cell_count: usize, cols: usize) -> usize {
    if cell_count == 0 {
        return cols.clamp(8, 40);
    }
    let chrome = cell_count.saturating_mul(3).saturating_add(1);
    cols.saturating_sub(chrome)
        .saturating_div(cell_count)
        .clamp(8, 32)
}

fn push_truncated_padded(out: &mut String, input: &str, width: usize) {
    let mut written = 0usize;
    for c in input.chars().take(width) {
        out.push(c);
        written += 1;
    }
    if input.chars().count() > width && width > 0 {
        out.pop();
        out.push('>');
    }
    while written < width {
        out.push(' ');
        written += 1;
    }
}

fn push_blank_line(out: &mut Vec<BrowserLine>) {
    if out.last().map(|line| !line.text.is_empty()).unwrap_or(true) {
        out.push(line(""));
    }
}

fn tag_is_hidden(lower_tag: &str) -> bool {
    let name = tag_name_of(lower_tag);
    let attrs = lower_tag[name.len()..].trim();
    // "hidden" as a standalone boolean attribute, not aria-hidden or data-hidden
    let has_hidden_attr = attrs
        .split(|c: char| c.is_ascii_whitespace())
        .any(|token| token == "hidden" || token.starts_with("hidden="));
    let has_hidden_class = attr_value(lower_tag, "class")
        .map(|classes| {
            classes.split_whitespace().any(|class| {
                matches!(
                    class,
                    "hidden" | "visually-hidden" | "sr-only" | "screen-reader-text"
                )
            })
        })
        .unwrap_or(false);
    has_hidden_attr
        || has_hidden_class
        || lower_tag.contains("display:none")
        || lower_tag.contains("display: none")
        || lower_tag.contains("visibility:hidden")
        || lower_tag.contains("visibility: hidden")
}

fn tag_name_of(lower_tag: &str) -> &str {
    lower_tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_raw_text_suppressed_element(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "noscript"
            | "svg"
            | "canvas"
            | "template"
            | "iframe"
            | "video"
            | "audio"
            | "object"
            | "embed"
            | "head"
    )
}

fn skip_raw_text_element(body: &str, lower_body: &str, content_start: usize, name: &str) -> usize {
    if is_void_element(name) {
        return content_start;
    }
    let mut close = String::from("</");
    close.push_str(name);
    if let Some(close_rel) = lower_body[content_start..].find(&close) {
        let close_start = content_start + close_rel;
        find_tag_end(&body[close_start..])
            .map(|close_end| close_start + close_end + 1)
            .unwrap_or(body.len())
    } else {
        body.len()
    }
}

fn closing_tag_for(lower_tag: &str) -> String {
    let mut out = String::from("/");
    out.push_str(tag_name_of(lower_tag));
    out
}

fn flush_flow_text(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
) {
    let mut prefix = state.pending_prefix.take();
    if state.quote_depth > 0 && state.kind != BrowserLineKind::Code {
        let mut quote = String::new();
        for _ in 0..state.quote_depth.min(3) {
            quote.push_str("> ");
        }
        if let Some(existing) = prefix {
            quote.push_str(&existing);
        }
        prefix = Some(quote);
    }
    let kind =
        if state.quote_depth > 0 && state.kind == BrowserLineKind::Text && state.link.is_none() {
            BrowserLineKind::Quote
        } else {
            state.kind
        };
    flush_text(
        out,
        text,
        cols,
        state.link.clone(),
        kind,
        prefix,
        state.current_align(),
        state.current_line_style(),
    );
}

fn flush_text(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
    prefix: Option<String>,
    align: BrowserAlign,
    style: BrowserLineStyle,
) {
    let trimmed = if kind == BrowserLineKind::Code {
        text.trim_matches('\n')
    } else {
        text.trim()
    };
    let mut decoded = decode_entities(trimmed);
    text.clear();
    if decoded.is_empty() {
        return;
    }
    if let Some(prefix) = prefix {
        decoded.insert_str(0, &prefix);
    }
    out.extend(wrap_plain_text_kind(
        &decoded, cols, link, kind, align, style,
    ));
}

fn clean_inline_text(input: &str) -> String {
    let mut out = String::new();
    for word in input.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn wrap_plain_text(text: &str, cols: usize, link: Option<String>) -> Vec<BrowserLine> {
    wrap_plain_text_kind(
        text,
        cols,
        link,
        BrowserLineKind::Text,
        BrowserAlign::Left,
        BrowserLineStyle::default(),
    )
}

fn wrap_plain_text_kind(
    text: &str,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
    align: BrowserAlign,
    style: BrowserLineStyle,
) -> Vec<BrowserLine> {
    let cols = style.content_cols(cols.clamp(20, 120)).clamp(8, 120);
    let mut chunks = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if word.len() > cols {
            if !line.is_empty() {
                chunks.push(line);
                line = String::new();
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.len() >= cols {
                    chunks.push(chunk);
                    chunk = String::new();
                }
                chunk.push(c);
            }
            if !chunk.is_empty() {
                line = chunk;
            }
            continue;
        }
        let extra = if line.is_empty() { 0 } else { 1 };
        if line.len() + word.len() + extra > cols && !line.is_empty() {
            chunks.push(line);
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        chunks.push(line);
    }
    let total = chunks.len();
    let mut out = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let part = if total <= 1 || !style.box_style.has_layout() {
            BrowserLineBoxPart::Single
        } else if idx == 0 {
            BrowserLineBoxPart::First
        } else if idx + 1 == total {
            BrowserLineBoxPart::Last
        } else {
            BrowserLineBoxPart::Middle
        };
        out.push(
            BrowserLine::new(chunk, link.clone(), line_kind_for_link(&link, kind))
                .aligned(align)
                .styled(style)
                .with_box_part(part),
        );
    }
    out
}

fn line_kind_for_link(link: &Option<String>, fallback: BrowserLineKind) -> BrowserLineKind {
    if link.is_some() {
        BrowserLineKind::Link
    } else {
        fallback
    }
}

fn is_separator_only(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    t.chars()
        .all(|c| matches!(c, '-' | '|' | '·' | '•' | '/' | '\\' | '_'))
}

fn compact_lines(lines: Vec<BrowserLine>) -> Vec<BrowserLine> {
    let mut out = Vec::new();
    let mut last_blank = false;
    for line in lines {
        let blank = line.text.trim().is_empty();
        if blank && last_blank {
            continue;
        }
        // Drop separator-only lines (e.g. lone "-" between footer links) with no link
        if line.link.is_none() && is_separator_only(&line.text) {
            continue;
        }
        last_blank = blank;
        out.push(line);
    }
    out
}

fn push_text_char(out: &mut String, c: char, preformatted: bool) {
    if preformatted && (c == '\n' || c == '\r') {
        if !out.ends_with('\n') {
            out.push('\n');
        }
    } else if c == '\n' || c == '\r' || c == '\t' {
        if !out.ends_with(' ') {
            out.push(' ');
        }
    } else {
        out.push(c);
    }
}

fn decode_entities(input: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if input[i..].starts_with("&nbsp;") {
                out.push(' ');
                i += 6;
                continue;
            }
            if input[i..].starts_with("&amp;") {
                out.push('&');
                i += 5;
                continue;
            }
            if input[i..].starts_with("&lt;") {
                out.push('<');
                i += 4;
                continue;
            }
            if input[i..].starts_with("&gt;") {
                out.push('>');
                i += 4;
                continue;
            }
            if input[i..].starts_with("&quot;") {
                out.push('"');
                i += 6;
                continue;
            }
            if input[i..].starts_with("&#39;") {
                out.push('\'');
                i += 5;
                continue;
            }
            if input[i..].starts_with("&#x") || input[i..].starts_with("&#X") {
                if let Some(end) = input[i + 3..].find(';') {
                    if let Some(value) = parse_entity_number(&input[i + 3..i + 3 + end], 16) {
                        out.push(value);
                        i += end + 4;
                        continue;
                    }
                }
            }
            if input[i..].starts_with("&#") {
                if let Some(end) = input[i + 2..].find(';') {
                    if let Some(value) = parse_entity_number(&input[i + 2..i + 2 + end], 10) {
                        out.push(value);
                        i += end + 3;
                        continue;
                    }
                }
            }
            // generic named entity fallback (&copy; &mdash; etc.)
            if let Some(semi) = input[i + 1..].find(';') {
                if semi < 24 {
                    let name = &input[i + 1..i + 1 + semi];
                    if name.bytes().all(|b| b.is_ascii_alphabetic()) {
                        if let Some(s) = named_entity_str(name) {
                            out.push_str(s);
                            i += semi + 2;
                            continue;
                        }
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn named_entity_str(name: &str) -> Option<&'static str> {
    match name {
        "copy" => Some("(c)"),
        "reg" => Some("(R)"),
        "trade" => Some("(TM)"),
        "apos" => Some("'"),
        "lsquo" | "rsquo" | "sbquo" => Some("'"),
        "ldquo" | "rdquo" | "bdquo" => Some("\""),
        "ndash" | "minus" => Some("-"),
        "mdash" => Some("--"),
        "hellip" => Some("..."),
        "bull" | "middot" => Some("*"),
        "laquo" => Some("<<"),
        "raquo" => Some(">>"),
        "euro" => Some("EUR"),
        "pound" => Some("GBP"),
        "yen" => Some("JPY"),
        "cent" => Some("c"),
        "deg" => Some("deg"),
        "times" => Some("x"),
        "divide" => Some("/"),
        "plusmn" => Some("+/-"),
        "rarr" | "rArr" => Some("->"),
        "larr" | "lArr" => Some("<-"),
        "harr" | "hArr" => Some("<->"),
        "uarr" => Some("^"),
        "darr" => Some("v"),
        "frac12" => Some("1/2"),
        "frac14" => Some("1/4"),
        "frac34" => Some("3/4"),
        "sup2" => Some("^2"),
        "sup3" => Some("^3"),
        "alpha" => Some("alpha"),
        "beta" => Some("beta"),
        "gamma" => Some("gamma"),
        "pi" => Some("pi"),
        "infin" => Some("inf"),
        "ne" => Some("!="),
        "le" => Some("<="),
        "ge" => Some(">="),
        "and" => Some("&&"),
        "or" => Some("||"),
        _ => None,
    }
}

fn parse_entity_number(input: &str, radix: u32) -> Option<char> {
    let mut value = 0u32;
    for b in input.bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u32,
            b'a'..=b'f' => (b - b'a' + 10) as u32,
            b'A'..=b'F' => (b - b'A' + 10) as u32,
            _ => return None,
        };
        if digit >= radix {
            return None;
        }
        value = value.checked_mul(radix)?.checked_add(digit)?;
        if value > 0x10_ffff {
            return None;
        }
    }
    char::from_u32(value)
}

fn extract_title(response: &str) -> Option<String> {
    let lower = lowercase_ascii(response);
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    Some(decode_entities(response[start..end].trim()))
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let name_bytes = name.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() {
        while pos < bytes.len()
            && (bytes[pos].is_ascii_whitespace() || matches!(bytes[pos], b'/' | b'<'))
        {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
        {
            pos += 1;
        }
        if start == pos {
            pos = pos.saturating_add(1);
            continue;
        }
        let key = &bytes[start..pos];
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b'=') {
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let value = if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
            let quote = bytes[pos];
            pos += 1;
            let value_start = pos;
            while pos < bytes.len() && bytes[pos] != quote {
                pos += 1;
            }
            String::from(&tag[value_start..pos])
        } else {
            let value_start = pos;
            while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                pos += 1;
            }
            String::from(&tag[value_start..pos])
        };
        if ascii_bytes_eq_ignore_case(key, name_bytes) {
            return Some(value);
        }
    }
    None
}

fn has_attr(tag: &str, name: &str) -> bool {
    let bytes = tag.as_bytes();
    let name_bytes = name.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() {
        while pos < bytes.len()
            && (bytes[pos].is_ascii_whitespace() || matches!(bytes[pos], b'/' | b'<'))
        {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
        {
            pos += 1;
        }
        if start == pos {
            pos = pos.saturating_add(1);
            continue;
        }
        if ascii_bytes_eq_ignore_case(&bytes[start..pos], name_bytes) {
            return true;
        }
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'=') {
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                pos = pos.saturating_add(1);
            } else {
                while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                    pos += 1;
                }
            }
        }
    }
    false
}

fn resolve_url(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.starts_with("browser://")
        || href.starts_with("file://")
        || href.starts_with("http://")
        || href.starts_with("https://")
    {
        return String::from(href);
    }
    if href.starts_with("//") {
        let scheme = if base.starts_with("https://") {
            "https:"
        } else if base.starts_with("http://") {
            "http:"
        } else {
            "https:"
        };
        let mut out = String::from(scheme);
        out.push_str(href);
        return out;
    }
    if href.starts_with('#') {
        let mut out = String::from(base);
        if let Some(hash) = out.find('#') {
            out.truncate(hash);
        }
        out.push_str(href);
        return out;
    }
    if let Some(path) = base.strip_prefix("file://") {
        let resolved = if href.starts_with('/') {
            crate::vfs::normalize_path(href)
        } else {
            let dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
            let mut joined = String::new();
            if dir.is_empty() {
                joined.push('/');
            } else {
                joined.push_str(dir);
                joined.push('/');
            }
            joined.push_str(href);
            crate::vfs::normalize_path(&joined)
        };
        return file_url_for_path(&resolved);
    }
    let Ok((scheme, host, path)) = parse_web_url(base) else {
        return normalize_url(href);
    };
    if href.starts_with('/') {
        let mut out = scheme;
        out.push_str("://");
        out.push_str(&host);
        out.push_str(href);
        return out;
    }
    let mut dir = path;
    if let Some(pos) = dir.rfind('/') {
        dir.truncate(pos + 1);
    }
    let mut out = scheme;
    out.push_str("://");
    out.push_str(&host);
    out.push_str(&dir);
    out.push_str(href);
    out
}

fn storage_origin_for_url(url: &str) -> Option<String> {
    if url.starts_with("file://") {
        return Some(String::from("file://"));
    }
    let Ok((scheme, host, _)) = parse_web_url(url) else {
        return None;
    };
    let mut out = scheme;
    out.push_str("://");
    out.push_str(&lowercase_ascii(&host));
    Some(out)
}

fn location_search(url: &str) -> String {
    let query_start = url.find('?');
    let Some(start) = query_start else {
        return String::new();
    };
    let end = url[start..]
        .find('#')
        .map(|rel| start + rel)
        .unwrap_or(url.len());
    String::from(&url[start..end])
}

fn find_tag_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => return Some(i),
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
            }
            b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn lowercase_ascii(input: &str) -> String {
    input
        .bytes()
        .map(|b| if b.is_ascii_uppercase() { b + 32 } else { b } as char)
        .collect()
}

fn ascii_bytes_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(l, r)| l.to_ascii_lowercase() == r.to_ascii_lowercase())
}

fn truncate_chars(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    let mut out = String::new();
    for c in s.chars().take(max.saturating_sub(1)) {
        out.push(c);
    }
    out.push('>');
    *s = out;
}
