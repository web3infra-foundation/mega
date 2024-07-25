// policy "anyoneCanViewPublicRepo" even if not login?
permit (
    principal,
    action in [Action::"viewRepo"],
    resource
)
unless { resource.is_private };

// policy "adminCanManageTeams"
// permit (
//     principal,
//     action in
//         [Action::"createTeam",
//          Action::"deleteTeam",
//          Action::"viewTeam",
//          Action::"addMember",
//          Action::"removeMember"],
//     resource
// )
// when { principal.role == "admin" };

// policy "memberCanViewTeam"
permit (
    principal,
    action in [Action::"viewTeam"],
    resource
)
when { principal in resource.members };

//Actions for readers
permit (
    principal,
    action in [Action::"viewRepo", Action::"pullRepo", Action::"forkRepo"],
    resource
)
when { principal in resource.readers };

permit (
    principal,
    action in [Action::"openIssue", Action::"createMergeRequest"],
    resource
)
when { principal in resource.repo.readers };

permit (
    principal,
    action in [Action::"editIssue"],
    resource
)
when { principal in resource.repo.readers && principal == resource.owner };

//Actions for triagers
permit (
    principal,
    action == Action::"assignIssue",
    resource
)
when { principal in resource.repo.triagers };

//Actions for writers
permit (
    principal,
    action == Action::"pushRepo",
    resource
)
when { principal in resource.writers };

permit (
    principal,
    action in [Action::"editIssue", Action::"approveMergeRequest"],
    resource
)
when { principal in resource.repo.writers };

//Actions for maintainers
permit (
    principal,
    action in [Action::"deleteIssue", Action::"approveMergeRequest"],
    resource
)
when { principal in resource.repo.maintainers };

//Actions for admins
permit (
    principal,
    action in
        [Action::"addReader",
         Action::"addTriager",
         Action::"addWriter",
         Action::"addMaintainer",
         Action::"addAdmin",
         Action::"deleteRepo"],
    resource
)
when { principal in resource.admins };