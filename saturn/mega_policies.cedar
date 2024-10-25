// policy "anyoneCanViewPublicRepo"
permit (
    principal,
    action in
        [Action::"viewRepo",
         Action::"pullRepo",
         Action::"forkRepo",
         Action::"pushRepo",
         Action::"openIssue",
         Action::"createMergeRequest"],
    resource
)
unless { resource.is_private };

//Actions for readers
permit (
    principal,
    action in
        [Action::"viewRepo",
         Action::"pullRepo",
         Action::"forkRepo",
         Action::"pushRepo",
         Action::"openIssue",
         Action::"createMergeRequest"],
    resource
)
when { principal in resource.readers };

//Actions for maintainers
permit (
    principal,
    action in
        [Action::"editIssue",
         Action::"editMergeRequest",
         Action::"assignIssue",
         Action::"approveMergeRequest"],
    resource
)
when { principal in resource.maintainers };

//Actions for admins
permit (
    principal,
    action in
        [Action::"addMaintainer",
         Action::"addAdmin",
         Action::"deleteRepo",
         Action::"deleteIssue",
         Action::"deleteMergeRequest"],
    resource
)
when { principal in resource.admins };

// root admin can do anything
permit (principal, action, resource)
when { principal == User::"genedna" || principal == User::"benjamin-747" };