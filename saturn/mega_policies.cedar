// policy "anyoneCanViewPublicRepo"
permit (
    principal,
    action in
        [Action::"viewRepo",
         Action::"pullRepo",
         Action::"forkRepo",
         Action::"pushRepo"],
    resource
)
unless { resource.is_private };

permit (
    principal,
    action in [Action::"openIssue", Action::"createMergeRequest"],
    resource
)
unless { resource.repo.is_private };

//Actions for maintainers
permit (
    principal,
    action in [Action::"editIssue", Action::"editMergeRequest"],
    resource
)
when { principal in resource.repo.maintainers || principal == resource.owner };

permit (
    principal,
    action in [Action::"assignIssue", Action::"approveMergeRequest"],
    resource
)
when { principal in resource.repo.maintainers };

//Actions for admins
permit (
    principal,
    action in
        [Action::"viewRepo",
         Action::"pullRepo",
         Action::"forkRepo",
         Action::"pushRepo",
         Action::"addMaintainer",
         Action::"addAdmin",
         Action::"deleteRepo"],
    resource
)
when { principal in resource.admins };

permit (
    principal,
    action in
        [Action::"openIssue",
         Action::"createMergeRequest",
         Action::"deleteIssue",
         Action::"deleteMergeRequest"],
    resource
)
when { principal in resource.repo.admins };