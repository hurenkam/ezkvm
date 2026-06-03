use crate::runtime_config::ValidationIssue;

pub(super) fn enrich_validation_issues(
    yaml: &str,
    issues: Vec<ValidationIssue>,
) -> Vec<ValidationIssue> {
    let locator = YamlLocator::from_source(yaml);

    issues
        .into_iter()
        .map(|issue| locator.attach_context(issue))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceContext {
    line_number: usize,
    snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct YamlLine {
    line_number: usize,
    indent: usize,
    content: String,
}

impl YamlLine {
    fn trimmed(&self) -> &str {
        self.content.trim()
    }
}

#[derive(Debug, Clone)]
struct YamlLocator {
    lines: Vec<YamlLine>,
}

#[derive(Debug, Clone, Copy)]
struct Scope {
    start: usize,
    end: usize,
    parent_indent: Option<usize>,
}

impl Scope {
    fn root(end: usize) -> Self {
        Self {
            start: 0,
            end,
            parent_indent: None,
        }
    }
}

impl YamlLocator {
    fn from_source(yaml: &str) -> Self {
        let lines = yaml
            .lines()
            .enumerate()
            .map(|(index, raw_line)| {
                let indent = raw_line.chars().take_while(|ch| *ch == ' ').count();
                YamlLine {
                    line_number: index + 1,
                    indent,
                    content: raw_line.to_owned(),
                }
            })
            .collect();

        Self { lines }
    }

    fn attach_context(&self, mut issue: ValidationIssue) -> ValidationIssue {
        if issue.line_number.is_some() && issue.source_snippet.is_some() {
            return issue;
        }

        let Some(context) = self.locate_path_best_effort(&issue.path) else {
            return issue;
        };

        if issue.line_number.is_none() {
            issue = issue.with_line_number(context.line_number);
        }
        if issue.source_snippet.is_none() {
            issue = issue.with_source_snippet(context.snippet);
        }

        issue
    }

    fn locate_path_best_effort(&self, path: &str) -> Option<SourceContext> {
        let segments = parse_issue_path(path);
        if segments.is_empty() {
            return self
                .first_contentful_line()
                .map(|index| self.context_for_index(index));
        }

        let mut scope = Scope::root(self.lines.len());
        let mut anchor_index = self.first_contentful_line()?;

        for segment in segments {
            let next_index = match segment {
                PathSegment::Key(key) => self.find_key_in_scope(scope, &key),
                PathSegment::Index(target) => self.find_list_item_in_scope(scope, target),
            };

            let Some(index) = next_index else {
                return Some(self.context_for_index(anchor_index));
            };

            anchor_index = index;
            scope = self.child_scope(index);
        }

        Some(self.context_for_index(anchor_index))
    }

    fn first_contentful_line(&self) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| !is_ignorable_line(line.trimmed()))
    }

    fn scoped_content_indexes(&self, scope: Scope) -> impl Iterator<Item = usize> + '_ {
        (scope.start..scope.end).filter(move |index| {
            let line = &self.lines[*index];
            if is_ignorable_line(line.trimmed()) {
                return false;
            }

            if let Some(parent_indent) = scope.parent_indent {
                line.indent > parent_indent
            } else {
                true
            }
        })
    }

    fn find_key_in_scope(&self, scope: Scope, key: &str) -> Option<usize> {
        self.scoped_content_indexes(scope)
            .find(|index| line_key(self.lines[*index].trimmed()) == Some(key))
    }

    fn find_list_item_in_scope(&self, scope: Scope, target_index: usize) -> Option<usize> {
        let mut list_indexes = self
            .scoped_content_indexes(scope)
            .filter(|index| self.lines[*index].trimmed().starts_with('-'));

        let first_index = list_indexes.next()?;
        let direct_item_indent = self.lines[first_index].indent;

        if target_index == 0 {
            return Some(first_index);
        }

        list_indexes
            .filter(|index| self.lines[*index].indent == direct_item_indent)
            .nth(target_index - 1)
    }

    fn child_scope(&self, start_index: usize) -> Scope {
        let start_indent = self.lines[start_index].indent;
        let mut end = self.lines.len();

        for index in (start_index + 1)..self.lines.len() {
            let line = &self.lines[index];
            if is_ignorable_line(line.trimmed()) {
                continue;
            }
            if line.indent <= start_indent {
                end = index;
                break;
            }
        }

        Scope {
            start: start_index + 1,
            end,
            parent_indent: Some(start_indent),
        }
    }

    fn context_for_index(&self, index: usize) -> SourceContext {
        let mut snippet_indexes = Vec::new();

        if let Some(previous) = self.previous_contentful_line(index) {
            snippet_indexes.push(previous);
        }
        snippet_indexes.push(index);
        if let Some(next) = self.next_contentful_line(index) {
            snippet_indexes.push(next);
        }

        snippet_indexes.sort_unstable();
        snippet_indexes.dedup();

        let snippet = snippet_indexes
            .into_iter()
            .map(|line_index| {
                let line = &self.lines[line_index];
                format!("{:>4} | {}", line.line_number, line.content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        SourceContext {
            line_number: self.lines[index].line_number,
            snippet,
        }
    }

    fn previous_contentful_line(&self, start_index: usize) -> Option<usize> {
        (0..start_index)
            .rev()
            .find(|index| !is_ignorable_line(self.lines[*index].trimmed()))
    }

    fn next_contentful_line(&self, start_index: usize) -> Option<usize> {
        ((start_index + 1)..self.lines.len())
            .find(|index| !is_ignorable_line(self.lines[*index].trimmed()))
    }
}

fn parse_issue_path(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();

    for raw_segment in path.split('.') {
        if raw_segment.is_empty() || raw_segment == "$" {
            continue;
        }

        let mut remainder = raw_segment;
        while let Some(open_bracket) = remainder.find('[') {
            let key = &remainder[..open_bracket];
            if !key.is_empty() {
                segments.push(PathSegment::Key(key.to_owned()));
            }

            let Some(close_bracket) = remainder[open_bracket + 1..].find(']') else {
                break;
            };
            let index_text = &remainder[open_bracket + 1..open_bracket + 1 + close_bracket];
            if let Ok(index) = index_text.parse::<usize>() {
                segments.push(PathSegment::Index(index));
            }

            remainder = &remainder[open_bracket + close_bracket + 2..];
        }

        if !remainder.is_empty() {
            segments.push(PathSegment::Key(remainder.to_owned()));
        }
    }

    segments
}

fn line_key(trimmed: &str) -> Option<&str> {
    if is_ignorable_line(trimmed) {
        return None;
    }

    let candidate = if let Some(rest) = trimmed.strip_prefix('-') {
        rest.trim_start()
    } else {
        trimmed
    };

    let (key, _) = candidate.split_once(':')?;
    let key = key.trim();
    if key.is_empty() { None } else { Some(key) }
}

fn is_ignorable_line(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed.starts_with('#')
}
