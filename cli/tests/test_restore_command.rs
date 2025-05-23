// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::common::create_commit_with_files;
use crate::common::CommandOutput;
use crate::common::TestEnvironment;
use crate::common::TestWorkDir;

#[test]
fn test_restore() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    work_dir.write_file("file1", "a\n");
    work_dir.run_jj(["new"]).success();
    work_dir.write_file("file2", "b\n");
    work_dir.run_jj(["new"]).success();
    work_dir.remove_file("file1");
    work_dir.write_file("file2", "c\n");
    work_dir.write_file("file3", "c\n");

    // There is no `-r` argument
    let output = work_dir.run_jj(["restore", "-r=@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: `jj restore` does not have a `--revision`/`-r` option. If you'd like to modify
    the *current* revision, use `--from`. If you'd like to modify a *different* revision,
    use `--into` or `--changes-in`.
    [EOF]
    [exit status: 1]
    ");

    // Restores from parent by default
    let output = work_dir.run_jj(["restore"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kkmpptxz 370d81ea (empty) (no description set)
    Parent commit (@-)      : rlvkpnrz ef160660 (no description set)
    Added 1 files, modified 1 files, removed 1 files
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s"]);
    insta::assert_snapshot!(output, @"");

    // Can restore another revision from its parents
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["diff", "-s", "-r=@-"]);
    insta::assert_snapshot!(output, @r"
    A file2
    [EOF]
    ");
    let output = work_dir.run_jj(["restore", "-c=@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: kkmpptxz 5b361547 (conflict) (no description set)
    Parent commit (@-)      : rlvkpnrz b9b6011e (empty) (no description set)
    Added 0 files, modified 1 files, removed 0 files
    Warning: There are unresolved conflicts at these paths:
    file2    2-sided conflict including 1 deletion
    New conflicts appeared in 1 commits:
      kkmpptxz 5b361547 (conflict) (no description set)
    Hint: To resolve the conflicts, start by updating to it:
      jj new kkmpptxz
    Then use `jj resolve`, or edit the conflict markers in the file directly.
    Once the conflicts are resolved, you may want to inspect the result with `jj diff`.
    Then run `jj squash` to move the resolution into the conflicted commit.
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s", "-r=@-"]);
    insta::assert_snapshot!(output, @"");

    // Can restore this revision from another revision
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["restore", "--from", "@--"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kkmpptxz 1154634b (no description set)
    Parent commit (@-)      : rlvkpnrz ef160660 (no description set)
    Added 1 files, modified 0 files, removed 2 files
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s"]);
    insta::assert_snapshot!(output, @r"
    D file2
    [EOF]
    ");

    // Can restore into other revision
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["restore", "--into", "@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: kkmpptxz 3fcdcbf2 (empty) (no description set)
    Parent commit (@-)      : rlvkpnrz ad805965 (no description set)
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s"]);
    insta::assert_snapshot!(output, @"");
    let output = work_dir.run_jj(["diff", "-s", "-r", "@-"]);
    insta::assert_snapshot!(output, @r"
    D file1
    A file2
    A file3
    [EOF]
    ");

    // Can combine `--from` and `--into`
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["restore", "--from", "@", "--into", "@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: kkmpptxz 9c6f2083 (empty) (no description set)
    Parent commit (@-)      : rlvkpnrz f256040a (no description set)
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s"]);
    insta::assert_snapshot!(output, @"");
    let output = work_dir.run_jj(["diff", "-s", "-r", "@-"]);
    insta::assert_snapshot!(output, @r"
    D file1
    A file2
    A file3
    [EOF]
    ");

    // Can restore only specified paths
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["restore", "file2", "file3"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kkmpptxz 4ad35a2f (no description set)
    Parent commit (@-)      : rlvkpnrz ef160660 (no description set)
    Added 0 files, modified 1 files, removed 1 files
    [EOF]
    ");
    let output = work_dir.run_jj(["diff", "-s"]);
    insta::assert_snapshot!(output, @r"
    D file1
    [EOF]
    ");
}

// Much of this test is copied from test_resolve_command
#[test]
fn test_restore_conflicted_merge() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit_with_files(&work_dir, "base", &[], &[("file", "base\n")]);
    create_commit_with_files(&work_dir, "a", &["base"], &[("file", "a\n")]);
    create_commit_with_files(&work_dir, "b", &["base"], &[("file", "b\n")]);
    create_commit_with_files(&work_dir, "conflict", &["a", "b"], &[]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    conflict
    ├─╮
    │ ○  b
    ○ │  a
    ├─╯
    ○  base
    ◆
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.read_file("file"), @r"
    <<<<<<< Conflict 1 of 1
    %%%%%%% Changes from base to side #1
    -base
    +a
    +++++++ Contents of side #2
    b
    >>>>>>> Conflict 1 of 1 ends
    ");

    // Overwrite the file...
    work_dir.write_file("file", "resolution");
    insta::assert_snapshot!(work_dir.run_jj(["diff"]), @r"
    Resolved conflict in file:
       1     : <<<<<<< Conflict 1 of 1
       2     : %%%%%%% Changes from base to side #1
       3     : -base
       4     : +a
       5     : +++++++ Contents of side #2
       6     : b
       7     : >>>>>>> Conflict 1 of 1 ends
            1: resolution
    [EOF]
    ");

    // ...and restore it back again.
    let output = work_dir.run_jj(["restore", "file"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv 25a37060 conflict | (conflict) (empty) conflict
    Parent commit (@-)      : zsuskuln aa493daf a | a
    Parent commit (@-)      : royxmykx db6a4daf b | b
    Added 0 files, modified 1 files, removed 0 files
    Warning: There are unresolved conflicts at these paths:
    file    2-sided conflict
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.read_file("file"), @r"
    <<<<<<< Conflict 1 of 1
    %%%%%%% Changes from base to side #1
    -base
    +a
    +++++++ Contents of side #2
    b
    >>>>>>> Conflict 1 of 1 ends
    ");
    let output = work_dir.run_jj(["diff"]);
    insta::assert_snapshot!(output, @"");

    // The same, but without the `file` argument. Overwrite the file...
    work_dir.write_file("file", "resolution");
    insta::assert_snapshot!(work_dir.run_jj(["diff"]), @r"
    Resolved conflict in file:
       1     : <<<<<<< Conflict 1 of 1
       2     : %%%%%%% Changes from base to side #1
       3     : -base
       4     : +a
       5     : +++++++ Contents of side #2
       6     : b
       7     : >>>>>>> Conflict 1 of 1 ends
            1: resolution
    [EOF]
    ");

    // ... and restore it back again.
    let output = work_dir.run_jj(["restore"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv f2c82b9c conflict | (conflict) (empty) conflict
    Parent commit (@-)      : zsuskuln aa493daf a | a
    Parent commit (@-)      : royxmykx db6a4daf b | b
    Added 0 files, modified 1 files, removed 0 files
    Warning: There are unresolved conflicts at these paths:
    file    2-sided conflict
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.read_file("file"), @r"
    <<<<<<< Conflict 1 of 1
    %%%%%%% Changes from base to side #1
    -base
    +a
    +++++++ Contents of side #2
    b
    >>>>>>> Conflict 1 of 1 ends
    ");
}

#[test]
fn test_restore_restore_descendants() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit_with_files(&work_dir, "base", &[], &[("file", "base\n")]);
    create_commit_with_files(&work_dir, "a", &["base"], &[("file", "a\n")]);
    create_commit_with_files(
        &work_dir,
        "b",
        &["base"],
        &[("file", "b\n"), ("file2", "b\n")],
    );
    create_commit_with_files(&work_dir, "ab", &["a", "b"], &[("file", "ab\n")]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    ab
    ├─╮
    │ ○  b
    ○ │  a
    ├─╯
    ○  base
    ◆
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.read_file("file"), @"ab");

    // Commit "b" was not supposed to modify "file", restore it from its parent
    // while preserving its child commit content.
    let output = work_dir.run_jj(["restore", "-c", "b", "file", "--restore-descendants"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits (while preserving their content)
    Working copy  (@) now at: vruxwmqv bf5491a0 ab | ab
    Parent commit (@-)      : zsuskuln aa493daf a | a
    Parent commit (@-)      : royxmykx 3fd5aa05 b | b
    [EOF]
    ");

    // Check that "a", "b", and "ab" have their expected content by diffing them.
    // "ab" must have kept its content.
    insta::assert_snapshot!(work_dir.run_jj(["diff", "--from=a", "--to=ab", "--git"]), @r"
    diff --git a/file b/file
    index 7898192261..81bf396956 100644
    --- a/file
    +++ b/file
    @@ -1,1 +1,1 @@
    -a
    +ab
    diff --git a/file2 b/file2
    new file mode 100644
    index 0000000000..6178079822
    --- /dev/null
    +++ b/file2
    @@ -0,0 +1,1 @@
    +b
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.run_jj(["diff", "--from=b", "--to=ab", "--git"]), @r"
    diff --git a/file b/file
    index df967b96a5..81bf396956 100644
    --- a/file
    +++ b/file
    @@ -1,1 +1,1 @@
    -base
    +ab
    [EOF]
    ");
}

#[test]
fn test_restore_interactive() {
    let mut test_env = TestEnvironment::default();
    let diff_editor = test_env.set_up_fake_diff_editor();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit_with_files(&work_dir, "a", &[], &[("file1", "a1\n"), ("file2", "a2\n")]);
    create_commit_with_files(
        &work_dir,
        "b",
        &["a"],
        &[("file1", "b1\n"), ("file2", "b2\n"), ("file3", "b3\n")],
    );
    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @  zsuskuln test.user@example.com 2001-02-03 08:05:11 b c0745ce2
    │  b
    │  M file1
    │  M file2
    │  A file3
    ○  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 186caaef
    │  a
    │  A file1
    │  A file2
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");

    let diff_script = [
        "files-before file1 file2 file3",
        "files-after JJ-INSTRUCTIONS file1 file2",
        "reset file2",
        "dump JJ-INSTRUCTIONS instrs",
    ]
    .join("\0");
    std::fs::write(diff_editor, diff_script).unwrap();

    // Restore file1 and file3
    let output = work_dir.run_jj(["restore", "-i", "--from=@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: zsuskuln bccde490 b | b
    Parent commit (@-)      : rlvkpnrz 186caaef a | a
    Added 0 files, modified 1 files, removed 1 files
    [EOF]
    ");

    insta::assert_snapshot!(
        std::fs::read_to_string(test_env.env_root().join("instrs")).unwrap(), @r"
    You are restoring changes from: rlvkpnrz 186caaef a | a
    to commit: zsuskuln c0745ce2 b | b

    The diff initially shows all changes restored. Adjust the right side until it
    shows the contents you want for the destination commit.
    ");

    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @  zsuskuln test.user@example.com 2001-02-03 08:05:13 b bccde490
    │  b
    │  M file2
    ○  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 186caaef
    │  a
    │  A file1
    │  A file2
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");

    // Try again with --tool, which should imply --interactive
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["restore", "--tool=fake-diff-editor"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: zsuskuln 5921de19 b | b
    Parent commit (@-)      : rlvkpnrz 186caaef a | a
    Added 0 files, modified 1 files, removed 1 files
    [EOF]
    ");

    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @  zsuskuln test.user@example.com 2001-02-03 08:05:16 b 5921de19
    │  b
    │  M file2
    ○  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 186caaef
    │  a
    │  A file1
    │  A file2
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");
}

#[test]
fn test_restore_interactive_merge() {
    let mut test_env = TestEnvironment::default();
    let diff_editor = test_env.set_up_fake_diff_editor();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit_with_files(&work_dir, "a", &[], &[("file1", "a1\n")]);
    create_commit_with_files(&work_dir, "b", &[], &[("file2", "b1\n")]);
    create_commit_with_files(
        &work_dir,
        "c",
        &["a", "b"],
        &[("file1", "c1\n"), ("file2", "c2\n"), ("file3", "c3\n")],
    );
    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @    royxmykx test.user@example.com 2001-02-03 08:05:13 c 34042291
    ├─╮  c
    │ │  M file1
    │ │  M file2
    │ │  A file3
    │ ○  zsuskuln test.user@example.com 2001-02-03 08:05:11 b 29e70804
    │ │  b
    │ │  A file2
    ○ │  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 79c1b823
    ├─╯  a
    │    A file1
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");

    let diff_script = [
        "files-before file1 file2 file3",
        "files-after JJ-INSTRUCTIONS file1 file2",
        "reset file2",
        "dump JJ-INSTRUCTIONS instrs",
    ]
    .join("\0");
    std::fs::write(diff_editor, diff_script).unwrap();

    // Restore file1 and file3
    let output = work_dir.run_jj(["restore", "-i"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: royxmykx 72e0cbf4 c | c
    Parent commit (@-)      : rlvkpnrz 79c1b823 a | a
    Parent commit (@-)      : zsuskuln 29e70804 b | b
    Added 0 files, modified 1 files, removed 1 files
    [EOF]
    ");

    insta::assert_snapshot!(
        std::fs::read_to_string(test_env.env_root().join("instrs")).unwrap(), @r"
    You are restoring changes from: rlvkpnrz 79c1b823 a | a
                                    zsuskuln 29e70804 b | b
    to commit: royxmykx 34042291 c | c

    The diff initially shows all changes restored. Adjust the right side until it
    shows the contents you want for the destination commit.
    ");

    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @    royxmykx test.user@example.com 2001-02-03 08:05:15 c 72e0cbf4
    ├─╮  c
    │ │  M file2
    │ ○  zsuskuln test.user@example.com 2001-02-03 08:05:11 b 29e70804
    │ │  b
    │ │  A file2
    ○ │  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 79c1b823
    ├─╯  a
    │    A file1
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");
}

#[test]
fn test_restore_interactive_with_paths() {
    let mut test_env = TestEnvironment::default();
    let diff_editor = test_env.set_up_fake_diff_editor();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit_with_files(&work_dir, "a", &[], &[("file1", "a1\n"), ("file2", "a2\n")]);
    create_commit_with_files(
        &work_dir,
        "b",
        &["a"],
        &[("file1", "b1\n"), ("file2", "b2\n"), ("file3", "b3\n")],
    );
    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @  zsuskuln test.user@example.com 2001-02-03 08:05:11 b c0745ce2
    │  b
    │  M file1
    │  M file2
    │  A file3
    ○  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 186caaef
    │  a
    │  A file1
    │  A file2
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");

    let diff_script = [
        "files-before file1 file2",
        "files-after JJ-INSTRUCTIONS file1 file2",
        "reset file2",
    ]
    .join("\0");
    std::fs::write(diff_editor, diff_script).unwrap();

    // Restore file1 (file2 is reset by interactive editor)
    let output = work_dir.run_jj(["restore", "-i", "file1", "file2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: zsuskuln 7187da33 b | b
    Parent commit (@-)      : rlvkpnrz 186caaef a | a
    Added 0 files, modified 1 files, removed 0 files
    [EOF]
    ");

    let output = work_dir.run_jj(["log", "--summary"]);
    insta::assert_snapshot!(output, @r"
    @  zsuskuln test.user@example.com 2001-02-03 08:05:13 b 7187da33
    │  b
    │  M file2
    │  A file3
    ○  rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 186caaef
    │  a
    │  A file1
    │  A file2
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");
}

#[must_use]
fn get_log_output(work_dir: &TestWorkDir) -> CommandOutput {
    work_dir.run_jj(["log", "-T", "bookmarks"])
}
