// policy "anyoneCanViewPublicRepo" even if not login?
permit (
    principal,
    action in [
        Mega::Action::"viewRepo"
    ],
    resource
)
when { resource.private == false };

// policy A User can perform any action on a Repo they own
permit (
  principal,
  action,
  resource is Mega::Repository
)
when { context.authenticated == true && resource.owner == principal };


// policy "adminCanManageRepos"
permit (
    principal,
    action in
        [Mega::Action::"createRepo",
         Mega::Action::"deleteRepo",
         Mega::Action::"viewRepo"],
    resource
)
when { context.authenticated == true && principal.role == "admin" };


// policy "collaboratorCanCreatePR"
permit (
    principal,
    action in [Mega::Action::"createPullRequest"],
    resource
)
when { context.authenticated == true && principal in resource.collaborators };

// policy "ownerCanMergePR"
permit (
    principal,
    action in [Mega::Action::"mergePullRequest"],
    resource
)
when { context.authenticated == true && principal == resource.owner };


// policy "userCanManageIssues" 
permit (
    principal,
    action in [
        Mega::Action::"createIssue",
        Mega::Action::"closeIssue"
    ],
    resource
)
when { context.authenticated == true && principal == resource.owner };


//policy "anyoneCanViewIssues"
permit (
    principal,
    action in [Mega::Action::"viewIssue"],
    resource
);