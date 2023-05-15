# Mega - A Monorepo Platform Engine

Mega is developing a monorepo engine similar to Google's Piper using Rust, which will facilitate Git and trunk-based development on a large scale.

## Git Compatible

Git is a distributed version control system that stores file versions on local machines, enabling fast access and distributed collaboration. Though Git repositories can scale to 20TB for mid-sized companies, managing such large codebases can be difficult.

Mega allows using Git with monorepo. It lets you clone or pull any monorepo folder into a local Git repo and push changes back. Though Git is popular, it lacks monorepo support. Mega bridges this gap.

## Trunk-based Development

Trunk-based development is an ideal workflow for monorepo. Monorepo are large codebases managed centrally, and trunk-based development provides a simple approach to changes. It involves working on a single codebase, frequent commits and testing, and deployments. This helps identify issues early, ensuring code stability. By enabling consistency and integration, trunk-based development keeps monorepo manageable, helping developers collaborate efficiently on big projects.

## Getting Started

Coming soon...

## Contributing

Mega is an open-source Git-based monorepo platform for trunk-based development. The project relies on community contributions and aims to simplify getting started. To use Mega, clone the repo, install dependencies, and run tests. Pick an issue, make changes, and submit a pull request for community review.

To contribute to Mega, you should:

- Familiarize yourself with the [Code of Conduct](CODE-OF-CONDUCT.md). Mega has a strict policy against abusive, unethical, or illegal behavior.
- Review the [Contributing Guidelines](CONTRIBUTING.md). This document outlines the process for submitting bug reports, feature requests, and pull requests to Mega.
- Sign the [Developer Certificate of Origin](https://developercertificate.org) (DCO) by adding a `Signed-off-by` line to your commit messages. This certifies that you wrote or have the right to submit the code you are contributing to the project.
- Choose an issue to work on. Issues labeled `good first issue` are suitable for newcomers. You can also look for issues marked `help wanted`.
- Fork the Mega repository and create a branch for your changes.
- Make your changes and commit them with a clear commit message.
- Push your changes to GitHub and open a pull request.
- Respond to any feedback on your pull request. The Mega maintainers will review your changes and may request modifications before merging.
- Once your pull request is merged, you will be listed as a contributor in the project repository and documentation.

To comply with the requirements, contributors must include both a `Signed-off-by` line and a PGP signature in their commit messages. You can find more information about how to generate a PGP key [here](https://docs.github.com/en/github/authenticating-to-github/managing-commit-signature-verification/generating-a-new-gpg-key).

Git even has a `-s` command line option to append this automatically to your commit message, and `-S` to sign your commit with your PGP key. For example:

```bash
$ git commit -S -s -m 'This is my commit message'
```

### Rebase the branch

If you have a local git environment and meet the criteria below, one option is to rebase the branch and add your Signed-off-by lines in the new commits. Please note that if others have already begun work based upon the commits in this branch, this solution will rewrite history and may cause serious issues for collaborators (described in the git documentation under “The Perils of Rebasing”).

You should only do this if:

- You are the only author of the commits in this branch
- You are absolutely certain nobody else is doing any work based upon this branch
- There are no empty commits in the branch (for example, a DCO Remediation Commit which was added using `-allow-empty`)

To add your Signed-off-by line to every commit in this branch:

- Ensure you have a local copy of your branch by checking out the pull request locally via command line.
- In your local branch, run: `git rebase HEAD~1 --signoff`
- Force push your changes to overwrite the branch: `git push --force-with-lease origin main`

## License

Mega is licensed under this Licensed:

- MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

## References

1. [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
2. [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)
3. [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)
4. [Trunk Based Development](https://trunkbaseddevelopment.com/)
5. [Branching strategies: Git-flow vs trunk-based development](https://www.devbridge.com/articles/branching-strategies-git-flow-vs-trunk-based-development/)
6. [Monorepo.tools](https://monorepo.tools/)
7. [Google Open Source Third Party](https://opensource.google/documentation/reference/thirdparty)