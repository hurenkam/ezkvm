pub fn app_name() -> &'static str {
    "ezkvm"
}

#[cfg(test)]
mod tests {
    use super::app_name;

    #[test]
    fn app_name_matches_project_name() {
        assert_eq!(app_name(), "ezkvm");
    }
}
