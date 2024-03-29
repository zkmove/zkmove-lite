## On Contributing

To get the development installation with all the necessary dependencies for
linting, testing, and building the documentation, run the following:

```bash
git clone https://github.com/young-rocks/zkmove-lite.git
cd zkmove-lite
./scripts/dev_setup.sh
source $HOME/.cargo/env
cargo build
```

## Development Process

### Code Style, Hints, and Testing

All code formatting is enforced with [rustfmt](https://github.com/rust-lang/rustfmt) with a project-specific configuration and checked by GitHub action.

Rust code should follow the rust coding guidelines:

* [rust-coding-guidelines](https://github.com/Rust-Coding-Guidelines/rust-coding-guidelines)

### Developer Workflow

Changes to the project are proposed through pull requests. The general pull
request workflow is as follows:

1. Fork the repo and create a topic branch off of `master`.
2. If you have added code that should be tested, add unit tests.
3. Check by `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings`.
4. Make sure your local workspace is clean and all changed file has been committed.
5. Submit your pull request.
6. Waiting for the github action check to pass and responding to reviewer feedback.

#### How to update the pull request

If your pull request is out-of-date and needs to be updated because `master`
has advanced, you should rebase your branch on top of the latest main by
doing the following:

```bash
git fetch upstream
git checkout topic
git rebase -i upstream/master
```

You *should not* update your branch by merging the latest main into your
branch. Merge commits included in PRs tend to make it more difficult for the
reviewer to understand the change being made, especially if the merge wasn't
clean and needed conflicts to be resolved. As such, PRs with merge commits will
be rejected.

## Issues

We uses [GitHub issues](https://github.com/young-rocks/zkmove-lite/issues) to track
bugs. Please include necessary information and instructions to reproduce your
issue.