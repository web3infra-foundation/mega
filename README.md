# Mega - A Monorepo Platform Engine

Google has a monorepo system, __Piper__, with more than 100 TB of data. It's built on top of Google's infrastructure. The purpose of Mega is to mimic Piper's architecture to implement a monorepo engine. The engine will be compatible with Git and Trunk based development flow.

## Git Compatible

Git is a file system that utilizes content addressing and also functions as a distributed collaboration system. Each file in a given repository is stored on the machine's hard drive, which provides numerous benefits for performance and maintenance. However, managing a sizable code repository such as the 20TB repositories common to mid-sized organizations can be challenging. Despite this, Git remains the most popular version control system globally, and Mega seeks to bridge the gap between Git and Monorepo. Mega enables Git to clone or pull any folder from a monorepo into the local development environment as a Git repository and then push changes back.

## Trunk-based Development

Trunk-based Development is a software development practice that involves working on a single codebase or trunk. This practice encourages continuous integration and delivery by pushing changes to the trunk frequently. 

The most suitable development workflow for Monorepo is trunk-based development. Monorepo are large codebases that require a centralized approach to version control, and trunk-based development provides a simple and effective way to manage changes in such an environment. This approach involves working on a single codebase or trunk, encouraging frequent commits, testing, and deployments. With trunk-based development, developers can identify and resolve conflicts early in the development cycle, ensuring the stability of the codebase. By promoting consistency and facilitating frequent integration, Trunk-based development ensures that monorepo remain manageable, enabling developers to work efficiently and collaboratively on large-scale projects.

## Getting Started

## Contributing

This project enforce the [DCO](https://developercertificate.org).

Contributors sign-off that they adhere to these requirements by adding a Signed-off-by line to commit messages.

```bash
This is my commit message

Signed-off-by: Random J Developer <random@developer.example.org>
```

Git even has a -s command line option to append this automatically to your commit message:

```bash
$ git commit -s -m 'This is my commit message'
```

### Rebase the branch

If you have a local git environment and meet the criteria below, one option is to rebase the branch and add your Signed-off-by lines in the new commits. Please note that if others have already begun work based upon the commits in this branch, this solution will rewrite history and may cause serious issues for collaborators (described in the git documentation under "The Perils of Rebasing").

You should only do this if:

* You are the only author of the commits in this branch
* You are absolutely certain nobody else is doing any work based upon this branch
* There are no empty commits in the branch (for example, a DCO Remediation Commit which was added using `--allow-empty`)

To add your Signed-off-by line to every commit in this branch:
* Ensure you have a local copy of your branch by checking out the pull request locally via command line.
* In your local branch, run: `git rebase HEAD~1 --signoff`
* Force push your changes to overwrite the branch: `git push --force-with-lease origin main`

### How to test

#### How to write and run unit tests

```bash
cargo test -- --nocapture --test-threads=1
```

## License

Mega is licensed under this Licensed:

* MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

## References

1. [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
2. [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)
3. [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)
4. [Trunk Based Development](https://trunkbaseddevelopment.com)
5. [Branching strategies: Git-flow vs trunk-based development](https://www.devbridge.com/articles/branching-strategies-git-flow-vs-trunk-based-development/)
6. [Monorepo.tools](https://monorepo.tools)
7. [Google Open Source Third Party](https://opensource.google/documentation/reference/thirdparty)