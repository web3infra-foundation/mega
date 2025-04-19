//! Default yaml configuration file parser.

use super::{FileContentError, YamlTask, YamlTaskError};
use crate::{utils::file::load_file, Action, CommandAction, DagError, Parser, Task};
use std::{collections::HashMap, sync::Arc};
use yaml_rust::{Yaml, YamlLoader};

/// An implementation of [`Parser`]. It is the default yaml configuration file parser.
pub struct YamlParser;

impl YamlParser {
    /// Parses an item in the configuration file into a task.
    /// An item refers to:
    ///
    /// ```yaml
    ///    name: "Task 1"
    ///    after: [b, c]
    ///    cmd: echo a
    /// ```
    fn parse_one(
        &self,
        id: &str,
        item: &Yaml,
        specific_action: Option<Action>,
    ) -> Result<YamlTask, YamlTaskError> {
        // Get name first
        let name = item["name"]
            .as_str()
            .ok_or(YamlTaskError::NoNameAttr(id.to_owned()))?
            .to_owned();
        // precursors can be empty
        let mut precursors = Vec::new();
        if let Some(after_tasks) = item["after"].as_vec() {
            after_tasks
                .iter()
                .for_each(|task_id| precursors.push(task_id.as_str().unwrap().to_owned()));
        }

        if let Some(action) = specific_action {
            Ok(YamlTask::new(id, precursors, name, action))
        } else {
            let cmd = item["cmd"]
                .as_str()
                .ok_or(YamlTaskError::NoScriptAttr(name.clone()))?;
            Ok(YamlTask::new(
                id,
                precursors,
                name,
                Action::Structure(Arc::new(CommandAction::new(cmd))),
            ))
        }
    }
}

impl Parser for YamlParser {
    fn parse_tasks(
        &self,
        file: &str,
        specific_actions: HashMap<String, Action>,
    ) -> Result<Vec<Box<dyn Task>>, DagError> {
        let content = load_file(file).map_err(|e| DagError::ParserError(e.to_string()))?;
        self.parse_tasks_from_str(&content, specific_actions)
    }

    fn parse_tasks_from_str(
        &self,
        content: &str,
        mut specific_actions: HashMap<String, Action>,
    ) -> Result<Vec<Box<dyn Task>>, DagError> {
        // Parse Yaml
        let yaml_tasks =
            YamlLoader::load_from_str(content).map_err(FileContentError::IllegalYamlContent)?;
        if yaml_tasks.is_empty() {
            return Err(DagError::ParserError("No Tasks found".to_string()));
        }
        let yaml_tasks = yaml_tasks[0]["dagrs"]
            .as_hash()
            .ok_or(YamlTaskError::StartWordError)?;

        let mut tasks = Vec::with_capacity(yaml_tasks.len());
        let mut map = HashMap::with_capacity(yaml_tasks.len());
        // Read tasks
        for (v, w) in yaml_tasks {
            let id = v
                .as_str()
                .ok_or(DagError::ParserError("Invalid YAML Node Type".to_string()))?;
            let task = specific_actions.remove(id).map_or_else(
                || self.parse_one(id, w, None),
                |action| self.parse_one(id, w, Some(action)),
            )?;
            map.insert(id, task.id());
            tasks.push(task);
        }

        for task in tasks.iter_mut() {
            let mut pres = Vec::new();
            for pre in task.str_precursors() {
                if map.contains_key(&pre[..]) {
                    pres.push(map[&pre[..]]);
                } else {
                    return Err(YamlTaskError::NotFoundPrecursor(task.name().to_string()).into());
                }
            }
            task.init_precursors(pres);
        }

        Ok(tasks
            .into_iter()
            .map(|task| Box::new(task) as Box<dyn Task>)
            .collect())
    }
}
