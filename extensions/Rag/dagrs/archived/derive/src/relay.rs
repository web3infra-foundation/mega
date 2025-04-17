use proc_macro2::Ident;
use std::collections::{HashMap, HashSet};
use syn::{parse::Parse, Token};

pub(crate) struct Relay {
    pub(crate) task: Ident,
    pub(crate) successors: Vec<Ident>,
}

pub(crate) struct Task {
    pub(crate) task: Ident,
    pub(crate) precursors: Vec<Ident>,
}

pub(crate) struct Tasks(pub(crate) Vec<Relay>);

impl Parse for Tasks {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut relies = Vec::new();
        loop {
            let mut successors = Vec::new();
            let task = input.parse::<Ident>()?;
            input.parse::<syn::Token!(->)>()?;
            while !input.peek(Token!(,)) && !input.is_empty() {
                successors.push(input.parse::<Ident>()?);
            }
            let relay = Relay { task, successors };
            relies.push(relay);
            let _ = input.parse::<Token!(,)>();
            if input.is_empty() {
                break;
            }
        }
        Ok(Self(relies))
    }
}

impl Tasks {
    fn check_duplicate(&self) -> syn::Result<()> {
        let mut set = HashSet::new();
        for relay in self.0.iter() {
            let task = &relay.task;
            if !set.contains(task) {
                set.insert(task);
            } else {
                let err_msg = format!("Duplicate task definition! [{}]", task);
                return Err(syn::Error::new_spanned(task, err_msg));
            }
        }
        Ok(())
    }

    pub(crate) fn resolve_dependencies(self) -> syn::Result<Vec<Task>> {
        self.check_duplicate()?;
        let mut seq = Vec::new();
        let tasks: HashMap<Ident, Vec<Ident>> = self
            .0
            .into_iter()
            .map(|item| {
                seq.push(item.task.clone());
                (item.task, item.successors)
            })
            .collect();
        let res = seq
            .into_iter()
            .map(|item| {
                let mut pre = Vec::new();
                tasks.iter().for_each(|(k, v)| {
                    if v.iter().any(|ele| ele.eq(&item)) {
                        pre.push(k.clone());
                    }
                });
                Task {
                    task: item.clone(),
                    precursors: pre,
                }
            })
            .collect();
        Ok(res)
    }
}

fn init_tasks(tasks: &[Task]) -> proc_macro2::TokenStream {
    let mut token = proc_macro2::TokenStream::new();
    for task in tasks.iter() {
        let ident = &task.task;
        let name = ident.to_string();
        token.extend(quote::quote!(
            let mut #ident=dagrs::DefaultTask::new(#name);
        ));
    }
    token
}

fn init_precursors(tasks: &[Task]) -> proc_macro2::TokenStream {
    let mut token = proc_macro2::TokenStream::new();
    for task in tasks.iter() {
        let ident = &task.task;
        let mut pres_token = proc_macro2::TokenStream::new();
        task.precursors.iter().for_each(|item| {
            pres_token.extend(quote::quote!(
                &#item,
            ));
        });
        token.extend(quote::quote!(
            #ident.set_predecessors(&[#pres_token]);
        ));
    }
    token
}

pub(crate) fn generate_task(tasks: Vec<Task>) -> proc_macro2::TokenStream {
    let tasks_defined_token: proc_macro2::TokenStream = init_tasks(&tasks);
    let init_pres_token: proc_macro2::TokenStream = init_precursors(&tasks);
    let tasks_ident: Vec<Ident> = tasks.into_iter().map(|item| item.task).collect();
    quote::quote!({
        #tasks_defined_token
        #init_pres_token
        vec![#(#tasks_ident,)*]
    })
}
