// policy "anyoneCanViewPublicRepo" even if not login?
permit (
    principal,
    action in [Mega::Action::"viewRepo"],
    resource
)
unless { resource.private };

// policy "adminCanManageTeams"
permit (
    principal,
    action in
        [Mega::Action::"createTeam",
         Mega::Action::"deleteTeam",
         Mega::Action::"viewTeam",
         Mega::Action::"addMember",
         Mega::Action::"removeMember"],
    resource
)
when { context.authenticated == true && principal.role == "admin" };

// policy "memberCanViewTeam"
permit (
    principal,
    action in [Mega::Action::"viewTeam"],
    resource
)
when { context.authenticated == true && principal in resource.members };

//Actions for readers
permit (
    principal,
    action in [Mega::Action::"pullRepo", Mega::Action::"forkRepo"],
    resource
)
when { principal in resource.readers };

permit (
    principal,
    action in [Mega::Action::"openIssue", Mega::Action::"createMergeRequest"],
    resource
)
when { principal in resource.repo.readers };

permit (
    principal,
    action in [Mega::Action::"editIssue"],
    resource
)
when { principal in resource.repo.readers && principal == resource.owner };

//Actions for triagers
permit (
    principal,
    action == Mega::Action::"assignIssue",
    resource
)
when { principal in resource.repo.triagers };

//Actions for writers
permit (
    principal,
    action == Mega::Action::"pushRepo",
    resource
)
when { principal in resource.writers };

permit (
    principal,
    action in [Mega::Action::"editIssue", Mega::Action::"approveMergeRequest"],
    resource
)
when { principal in resource.repo.writers };

//Actions for maintainers
permit (
    principal,
    action in
        [Mega::Action::"deleteIssue", Mega::Action::"approveMergeRequest"],
    resource
)
when { principal in resource.repo.maintainers };

//Actions for admins
permit (
    principal,
    action in
        [Mega::Action::"addReader",
         Mega::Action::"addTriager",
         Mega::Action::"addWriter",
         Mega::Action::"addMaintainer",
         Mega::Action::"addAdmin"],
    resource
)
when { principal in resource.admins };