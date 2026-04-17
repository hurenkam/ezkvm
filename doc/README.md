# Documentation structure

- `<workspace>/doc/`: Top level of the documentation tree
  - `backlog/`: Contains documents that keep track of ongoing work and their progress.
    - `dev/`: Top level of developer documentation
        - `adr/`: Directory contains Architectural Design Rulings; design choices affecting major features
        - `ARCHITECTURAL_GUIDELINES.md`: architectural guidelines, to be used by developers or ai instructions/agents/skills to make architectural choices
        - `CODING_GUIDELINES.md`: coding guidelines, to be used by developers or ai instructions/agents/skills to keep the codebase clean
        - `CONTRIBUTING.md`: info for potential contributers on how they can help improve ezkvm
        - `MODULE_OWNERSHIP.md`:
    - `preparation/`: This is where feature preparation documents are stored.
    - `user/`: Top level of user level documentation
        - `config/`: Describe the configuration files: central config, profiles and vm config.
        - `cli.md`: Describe the command line commands and arguments
