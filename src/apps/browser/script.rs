fn script_stats_debug_line(stats: BrowserScriptStats) -> String {
    format!(
        "js inline={} external={}/{} handlers={} timers={} mutations={} storage={}/{} cookies={}/{} fetch={} nav={} errors={} statements={}",
        stats.inline_scripts,
        stats.external_scripts,
        stats.external_scripts.saturating_add(stats.external_failed),
        stats.handlers,
        stats.timers,
        stats.mutations,
        stats.storage_reads,
        stats.storage_writes,
        stats.cookie_reads,
        stats.cookie_writes,
        stats.fetches,
        stats.navigation_requests,
        stats.errors,
        stats.statements
    )
}

fn split_script_statements(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && out.len() < MAX_SCRIPT_STATEMENTS {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => brace_depth = brace_depth.saturating_add(1),
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if brace_depth == 0 => {
                out.push(String::from(input[start..i].trim()));
                start = i.saturating_add(1);
            }
            _ => {}
        }
        i += 1;
    }
    if start < input.len() && out.len() < MAX_SCRIPT_STATEMENTS {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            out.push(String::from(tail));
        }
    }
    out
}

fn script_statement_is_ignorable(statement: &str) -> bool {
    let trimmed = statement.trim();
    trimmed.is_empty()
        || trimmed == "\"use strict\""
        || trimmed == "'use strict'"
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || ((trimmed.starts_with("var ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const "))
            && !trimmed.contains('='))
}

fn compact_script_expr(input: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in input.chars() {
        if let Some(q) = quote {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            out.push(c);
        } else if !c.is_ascii_whitespace() {
            out.push(c);
        }
    }
    out
}

fn parse_add_event_listener(
    statement: &str,
) -> Option<(BrowserScriptTarget, BrowserScriptEvent, String)> {
    let marker = ".addEventListener";
    let pos = statement.find(marker)?;
    let target = parse_script_target(&statement[..pos])?;
    let args = &statement[pos + marker.len()..];
    let open = args.find('(')?;
    let args = &args[open + 1..];
    let (event_name, _) = parse_script_string_literal(args)?;
    let event = BrowserScriptEvent::from_name(&event_name)?;
    let body = extract_script_function_body(statement)?;
    Some((target, event, body))
}

fn split_script_assignment(statement: &str) -> Option<(&str, &str)> {
    let bytes = statement.as_bytes();
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'=' if depth == 0 => {
                let prev = i.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
                let next = bytes.get(i + 1).copied();
                if matches!(prev, Some(b'=' | b'!' | b'<' | b'>' | b'+') | Some(b'-'))
                    || matches!(next, Some(b'=' | b'>'))
                {
                    i += 1;
                    continue;
                }
                return Some((statement[..i].trim(), statement[i + 1..].trim()));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_script_assignment_left(
    left: &str,
) -> Option<(BrowserScriptTarget, BrowserScriptProperty)> {
    let compact = compact_script_expr(left);
    if let Some((target, property)) = parse_style_assignment_left(&compact) {
        return Some((target, BrowserScriptProperty::Style(property)));
    }
    for (suffix, property) in [
        (".textContent", BrowserScriptProperty::TextContent),
        (".innerText", BrowserScriptProperty::TextContent),
        (".className", BrowserScriptProperty::ClassName),
        (".value", BrowserScriptProperty::Value),
        (".checked", BrowserScriptProperty::Checked),
        (".disabled", BrowserScriptProperty::Disabled),
    ] {
        if let Some(target) = compact.strip_suffix(suffix) {
            return parse_script_target(target).map(|target| (target, property));
        }
    }
    None
}

fn parse_style_assignment_left(compact: &str) -> Option<(BrowserScriptTarget, String)> {
    let marker = ".style.";
    let pos = compact.rfind(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let property = css_property_from_js_name(&compact[pos + marker.len()..])?;
    Some((target, property))
}

fn parse_script_target(input: &str) -> Option<BrowserScriptTarget> {
    let compact = compact_script_expr(input);
    parse_script_call_string_arg(&compact, "document.getElementById")
        .map(BrowserScriptTarget::Id)
        .or_else(|| {
            parse_script_call_string_arg(&compact, "document.querySelector")
                .map(BrowserScriptTarget::Selector)
        })
        .or_else(|| parse_query_selector_all_target(&compact))
}

fn parse_query_selector_all_target(input: &str) -> Option<BrowserScriptTarget> {
    let close = input.find(")[")?;
    let target = &input[..close + 1];
    let selector = parse_script_call_string_arg(target, "document.querySelectorAll")?;
    let rest = &input[close + 2..];
    let end = rest.find(']')?;
    let index = rest[..end].parse::<usize>().ok()?;
    Some(BrowserScriptTarget::SelectorAll(selector, index))
}

fn parse_storage_call(compact: &str) -> Option<BrowserStorageCall> {
    let (area, rest) = strip_storage_area_prefix(compact)?;
    if let Some(args) = parse_method_args(rest, ".setItem") {
        let key = parse_script_string_literal(args.first()?.as_str())?.0;
        let value_expr = args.get(1)?.clone();
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::SetItem,
            key: Some(truncate_script_value(&key)),
            value_expr: Some(value_expr),
        });
    }
    if let Some(args) = parse_method_args(rest, ".removeItem") {
        let key = parse_script_string_literal(args.first()?.as_str())?.0;
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::RemoveItem,
            key: Some(truncate_script_value(&key)),
            value_expr: None,
        });
    }
    if rest == ".clear()" {
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::Clear,
            key: None,
            value_expr: None,
        });
    }
    None
}

fn parse_storage_get_item_expr(compact: &str) -> Option<(BrowserStorageArea, String)> {
    let (area, rest) = strip_storage_area_prefix(compact)?;
    let args = parse_method_args(rest, ".getItem")?;
    let key = parse_script_string_literal(args.first()?.as_str())?.0;
    Some((area, truncate_script_value(&key)))
}

fn strip_storage_area_prefix(input: &str) -> Option<(BrowserStorageArea, &str)> {
    for (prefix, area) in [
        ("localStorage", BrowserStorageArea::Local),
        ("window.localStorage", BrowserStorageArea::Local),
        ("sessionStorage", BrowserStorageArea::Session),
        ("window.sessionStorage", BrowserStorageArea::Session),
    ] {
        if let Some(rest) = input.strip_prefix(prefix) {
            return Some((area, rest));
        }
    }
    None
}

fn parse_class_list_call(compact: &str) -> Option<ClassListCall> {
    let marker = ".classList.";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let rest = &compact[pos + marker.len()..];
    for (name, op) in [
        ("add", ClassListOp::Add),
        ("remove", ClassListOp::Remove),
        ("toggle", ClassListOp::Toggle),
    ] {
        let Some(args) = parse_method_args(rest, name) else {
            continue;
        };
        let class_name = parse_script_string_literal(args.first()?.as_str())?.0;
        return Some(ClassListCall {
            target,
            op,
            class_name: truncate_script_value(&class_name),
        });
    }
    None
}

fn parse_attribute_call(compact: &str) -> Option<BrowserAttributeCall> {
    let marker = ".setAttribute";
    if let Some(pos) = compact.find(marker) {
        let target = parse_script_target(&compact[..pos])?;
        let args = parse_method_args(&compact[pos..], marker)?;
        let name = parse_script_string_literal(args.first()?.as_str())?.0;
        let value_expr = args.get(1)?.clone();
        return Some(BrowserAttributeCall {
            target,
            op: BrowserAttributeOp::Set,
            name: lowercase_ascii(&truncate_script_value(&name)),
            value_expr: Some(value_expr),
        });
    }
    let marker = ".removeAttribute";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let args = parse_method_args(&compact[pos..], marker)?;
    let name = parse_script_string_literal(args.first()?.as_str())?.0;
    Some(BrowserAttributeCall {
        target,
        op: BrowserAttributeOp::Remove,
        name: lowercase_ascii(&truncate_script_value(&name)),
        value_expr: None,
    })
}

fn parse_get_attribute_expr(compact: &str) -> Option<BrowserAttributeCall> {
    let marker = ".getAttribute";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let args = parse_method_args(&compact[pos..], marker)?;
    let name = parse_script_string_literal(args.first()?.as_str())?.0;
    Some(BrowserAttributeCall {
        target,
        op: BrowserAttributeOp::Set,
        name: lowercase_ascii(&truncate_script_value(&name)),
        value_expr: None,
    })
}

fn parse_history_url_arg(compact: &str) -> Option<String> {
    let prefixes = [
        "history.pushState",
        "window.history.pushState",
        "history.replaceState",
        "window.history.replaceState",
        "location.assign",
        "window.location.assign",
        "location.replace",
        "window.location.replace",
    ];
    for prefix in prefixes {
        if let Some(args) = parse_method_args(compact, prefix) {
            if prefix.contains("history.") {
                return args.get(2).cloned();
            }
            return args.first().cloned();
        }
    }
    None
}

fn parse_fetch_request(compact: &str) -> Option<BrowserFetchRequest> {
    let args = if compact.starts_with("fetch(") {
        parse_method_args(compact, "fetch")?
    } else {
        parse_method_args(compact, "window.fetch")?
    };
    let url = parse_script_string_literal(args.first()?.as_str())?.0;
    let mut method = BrowserFetchMethod::Get;
    let mut body = String::new();
    if let Some(options) = args.get(1) {
        if parse_object_string_field(options, "method")
            .map(|value| value.eq_ignore_ascii_case("POST"))
            .unwrap_or(false)
        {
            method = BrowserFetchMethod::Post;
        }
        if let Some(value) = parse_object_string_field(options, "body") {
            body = truncate_script_value(&value);
        }
    }
    Some(BrowserFetchRequest {
        url: truncate_script_value(&url),
        method,
        body,
    })
}

fn parse_object_string_field(input: &str, key: &str) -> Option<String> {
    let compact = compact_script_expr(input);
    for quote in ["'", "\""] {
        let marker = format!("{}:{}", key, quote);
        if let Some(pos) = compact.find(&marker) {
            let value_start = pos + marker.len() - quote.len();
            return parse_script_string_literal(&compact[value_start..]).map(|(value, _)| value);
        }
    }
    None
}

fn load_fetch_post_uncached(url: &str, body: &str) -> Result<BrowserFetchedResource, &'static str> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("unsupported fetch POST URL");
    }
    let response =
        crate::net::browser_post_response(url, body, "application/x-www-form-urlencoded")?;
    Ok(BrowserFetchedResource {
        final_url: response.final_url,
        content_type: response.content_type,
        bytes: response.body_bytes,
        cache_hit: false,
    })
}

fn extract_fetch_text_callback_body(statement: &str) -> Option<String> {
    let compact = compact_script_expr(statement);
    let marker = ".text().then";
    let compact_pos = compact.find(marker)?;
    let mut seen_non_ws = 0usize;
    let mut original_pos = 0usize;
    for (idx, c) in statement.char_indices() {
        if !c.is_ascii_whitespace() {
            if seen_non_ws == compact_pos {
                original_pos = idx;
                break;
            }
            seen_non_ws += 1;
        }
    }
    extract_script_function_body(&statement[original_pos..])
}

fn parse_method_args(input: &str, method: &str) -> Option<Vec<String>> {
    let rest = input.strip_prefix(method)?;
    let rest = rest.strip_prefix('(')?;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 1usize;
    let bytes = rest.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(split_script_args(&rest[..idx]));
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn split_script_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let bytes = input.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                out.push(String::from(input[start..idx].trim()));
                start = idx.saturating_add(1);
            }
            _ => {}
        }
    }
    if start <= input.len() {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            out.push(String::from(tail));
        }
    }
    out
}

fn split_script_concat(input: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let bytes = input.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'+' if depth == 0 => {
                out.push(String::from(input[start..idx].trim()));
                start = idx.saturating_add(1);
            }
            _ => {}
        }
    }
    if out.is_empty() {
        return None;
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        out.push(String::from(tail));
    }
    Some(out)
}

fn css_property_from_js_name(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() || input.len() > 48 {
        return None;
    }
    let mut out = String::new();
    for c in input.chars() {
        if c.is_ascii_uppercase() {
            if !out.is_empty() {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            out.push(c);
        } else {
            return None;
        }
    }
    Some(out)
}

fn valid_script_var_name(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && input.len() <= 32
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn set_style_declaration(style: &mut String, property: &str, value: &str) {
    let mut declarations: Vec<(String, String)> = Vec::new();
    let wanted = lowercase_ascii(property);
    let mut replaced = false;
    for part in style.split(';') {
        let Some((name, current)) = part.split_once(':') else {
            continue;
        };
        let name = lowercase_ascii(name.trim());
        if name.is_empty() {
            continue;
        }
        if name == wanted {
            declarations.push((name, String::from(value.trim())));
            replaced = true;
        } else {
            declarations.push((name, String::from(current.trim())));
        }
        if declarations.len() >= MAX_DOM_ATTRS {
            break;
        }
    }
    if !replaced && declarations.len() < MAX_DOM_ATTRS {
        declarations.push((wanted, String::from(value.trim())));
    }
    style.clear();
    for (idx, (name, value)) in declarations.iter().enumerate() {
        if idx > 0 {
            style.push(';');
        }
        style.push_str(name);
        style.push(':');
        style.push_str(value);
    }
}

fn style_declaration_value(style: &str, property: &str) -> Option<String> {
    let wanted = lowercase_ascii(property);
    for part in style.split(';') {
        let Some((name, value)) = part.split_once(':') else {
            continue;
        };
        if lowercase_ascii(name.trim()) == wanted {
            return Some(String::from(value.trim()));
        }
    }
    None
}

fn parse_script_call_string_arg(input: &str, name: &str) -> Option<String> {
    let rest = input.strip_prefix(name)?;
    let args = rest.strip_prefix('(')?.strip_suffix(')')?;
    parse_script_string_literal(args).map(|(value, _)| value)
}

fn parse_script_string_value(input: &str) -> Option<String> {
    parse_script_string_literal(input.trim()).map(|(value, _)| truncate_script_value(&value))
}

fn parse_script_bool_value(input: &str) -> Option<bool> {
    match lowercase_ascii(input.trim()).as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_script_string_literal(input: &str) -> Option<(String, usize)> {
    let input = input.trim_start();
    let bytes = input.as_bytes();
    let quote = *bytes.first()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == quote {
            return Some((out, i + 1));
        }
        if b == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'\'' => out.push('\''),
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                _ => out.push(next as char),
            }
            i += 2;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    None
}

fn extract_script_function_body(statement: &str) -> Option<String> {
    let bytes = statement.as_bytes();
    let mut open = None;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => {
                open = Some(idx);
                break;
            }
            _ => {}
        }
    }
    let open = open?;
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for idx in open..bytes.len() {
        let b = bytes[idx];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(String::from(&statement[open + 1..idx]));
                }
            }
            _ => {}
        }
    }
    None
}

fn truncate_script_value(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        if out.len().saturating_add(c.len_utf8()) > MAX_FORM_VALUE {
            break;
        }
        out.push(c);
    }
    out
}

fn push_html_text_escaped(out: &mut String, input: &str) {
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn push_html_attr_escaped(out: &mut String, input: &str) {
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn push_hex_color(out: &mut String, color: u32) {
    out.push('#');
    for shift in [20u32, 16, 12, 8, 4, 0] {
        out.push(hex_digit(((color >> shift) & 0x0f) as u8));
    }
}
