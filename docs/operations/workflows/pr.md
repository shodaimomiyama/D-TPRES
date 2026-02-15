---
name: pr-workflow
description: Pull Request作成からマージまでのワークフロー
keywords:
  - pr
  - pull request
  - プルリクエスト
  - code review
  - コードレビュー
  - review
  - レビュー
  - github
  - git
  - merge
  - マージ
---

# PR Creation Workflow

## Overview

This document defines the rules and procedures for creating GitHub Pull Requests.

## Important Notes

- **Worktree Users:** When using git worktrees, ensure that you only add and commit changes from your current worktree. Do not accidentally stage changes from other worktrees to avoid mixing unrelated changes in your PR.

- **Use Japanese:** 日本語で PR は作成してください

## Steps

1. **Check Changes:** Verify that all necessary changes have been added and committed:
    - Run `git status` to check for any uncommitted changes
    - Run `git diff` to review unstaged changes
    - Add necessary files with `git add` and commit them
    - Ensure no unintended files from other worktrees are included
2. **Working Directory:** Ensure you are in the root directory (`/`). Change to the root directory if you are not.
3. **Confirm Implementation Details:** Review all commit messages from the develop branch divergence point to understand and summarize the changes made in this PR.
4. **Git Push:** Run the command `git push`.
5. **Create PR Summary:**
    - Create the Pull Request summary following the format in `.github/pull_request_template.md`.
    - The AI will verify the checklist items itself; user confirmation for the checklist is not required.
6. **Get and Select Labels:**
    1. Search labels to get the list of available labels. Select a minimum of one and a maximum of three labels relevant to the current implementation.
7. **Get and Select Issues:**
    1. Search issues to get the list of currently open issues. Select the issue(s) relevant to the current implementation.
    2. If there are no relevant issues, use `ISSUES="none"` when executing the PR creation command.
8. **Confirm PR Content and Save Temporarily:**
    - Ask the user to review the created PR content (summary) for any necessary corrections.
    - After confirmation, temporarily save the content to `.github/pr_description.md`.
9. **Execute PR Creation Command:**
    - Create PR TITLE="<PR Title>" LABELS="<Label1>,<Label2>" ISSUES="<Issue1>,<Issue2>".
    - Do not include the `#` sign in the issue number.
    - The PR should be automatically assigned to the current user (`@me`) by default.
