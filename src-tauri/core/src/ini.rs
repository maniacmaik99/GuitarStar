//! Minimal INI editor that keeps comments, ordering and line endings intact,
//! so files written by the game or by RS_ASIO stay recognisable after edits.

#[derive(Debug, Clone)]
pub struct IniDoc {
    lines: Vec<String>,
    newline: &'static str,
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
        Some(t[1..t.len() - 1].trim())
    } else {
        None
    }
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    if t.starts_with(';') || t.starts_with('#') {
        return None;
    }
    let (k, v) = t.split_once('=')?;
    Some((k.trim(), v.trim()))
}

impl IniDoc {
    pub fn parse(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        IniDoc {
            lines: text.lines().map(String::from).collect(),
            newline,
        }
    }

    /// Line range `(header, end)` of a section; `end` is exclusive.
    fn section_range(&self, section: &str) -> Option<(usize, usize)> {
        let start = self
            .lines
            .iter()
            .position(|l| section_name(l).is_some_and(|s| s.eq_ignore_ascii_case(section)))?;
        let end = self.lines[start + 1..]
            .iter()
            .position(|l| section_name(l).is_some())
            .map_or(self.lines.len(), |i| start + 1 + i);
        Some((start, end))
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let (start, end) = self.section_range(section)?;
        self.lines[start + 1..end]
            .iter()
            .filter_map(|l| entry(l))
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.to_string())
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let new_line = format!("{key}={value}");
        let Some((start, end)) = self.section_range(section) else {
            if self.lines.last().is_some_and(|l| !l.trim().is_empty()) {
                self.lines.push(String::new());
            }
            self.lines.push(format!("[{section}]"));
            self.lines.push(new_line);
            return;
        };
        let existing = (start + 1..end)
            .find(|&i| entry(&self.lines[i]).is_some_and(|(k, _)| k.eq_ignore_ascii_case(key)));
        match existing {
            Some(i) => self.lines[i] = new_line,
            None => {
                // Insert after the last entry so trailing blank lines stay between sections.
                let last_entry = (start + 1..end)
                    .rev()
                    .find(|&i| entry(&self.lines[i]).is_some())
                    .unwrap_or(start);
                self.lines.insert(last_entry + 1, new_line);
            }
        }
    }

    pub fn render(&self) -> String {
        let mut out = self.lines.join(self.newline);
        out.push_str(self.newline);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_comments_and_updates_values() {
        let src = "; comment\r\n[Audio]\r\nLatencyBuffer=4\r\n\r\n[Renderer.Win32]\r\nScreenWidth=1280\r\n";
        let mut doc = IniDoc::parse(src);
        doc.set("audio", "latencybuffer", "2");
        doc.set("Audio", "ExclusiveMode", "1");
        doc.set("Renderer.Win32", "ScreenWidth", "2560");
        assert_eq!(
            doc.render(),
            "; comment\r\n[Audio]\r\nlatencybuffer=2\r\nExclusiveMode=1\r\n\r\n[Renderer.Win32]\r\nScreenWidth=2560\r\n"
        );
        assert_eq!(doc.get("Audio", "ExclusiveMode").as_deref(), Some("1"));
    }

    #[test]
    fn adds_missing_section() {
        let mut doc = IniDoc::parse("[Config]\nEnableAsio=1\n");
        doc.set("Asio.Output", "Driver", "Focusrite USB ASIO");
        assert_eq!(doc.render(), "[Config]\nEnableAsio=1\n\n[Asio.Output]\nDriver=Focusrite USB ASIO\n");
    }

    #[test]
    fn ignores_commented_keys() {
        let doc = IniDoc::parse("[A]\n;Driver=x\n");
        assert_eq!(doc.get("A", "Driver"), None);
    }
}
