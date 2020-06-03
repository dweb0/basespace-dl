# TODO

* Automatically get token ID when new token is added
* Implement "cache" for storing http responses
* Add ability to fetch run instead of project (on hold, v1pre3/runs/{run}/samples does not seem to be working on basespace's end)
* Add etag verification to ensure file downloaded correctly
* Unit tests, especially for s3 etag

# 0.4.0  (2020-05-20)

* Separated logic between getting samples and files, and separated the Undetermined file logic into a new function called `get_undetermined_sample`.
* Updated all crates to latest version (lot of the current async / await dependencies were still in alpha).

# 0.3.1  (2020-03-30)

* Because Illumina strips trailing whitespace in the name field of `v1pre3/users/current/projects`, but not in the ExperimentName field of `v1pre3/projects/${id_of_unindexed_reads_project}/samples`, added an extra check to work around this.

# 0.3.0  (2019-12-07)

* Complete rewrite with async/await!
* Better error handling in `main.rs`.
* Better API deserialization using `serde_struct_wrapper`. No longer need to use wrapper structs
* Added jaro_winkler to show similar projects if queried project does not exist
* Now uses elastic tabstops to display tab separated output
* Better file output formatting with progress bar

# 0.2.3  (2019-10-16)

* Added check for Undetermined files - if a project is rerun, basespace will generate a new "Unindexed Reads"
sample under the same project. The only way to tell the difference is by date, so I added a prompt to let
the user choose if this happens.
* Updated `Cargo.toml` to use most recent crates.

# 0.2.2  (2019-10-07)

* Added file size check
* Support for multiple config files

# 0.2.1  (2019-07-21)

* Can now download projects not owned by self (e.g. shared projects or publicly available datasets linked to your account)
* Better logging (added verbose option)
* Added build scripts to cross compile

# 0.2.0  (2019-07-20)

* Files are now downloaded concurrently. Woohoo!
    - Also prints average speed when finished
* Added select-files argument: specify the exact file names you want
    - Useful for chaining basespace-dl commands (pipe one output to another)
* Fixed filter method to use the file names instead of sample names
* Lots of minor code cleanups

# 0.1.2  (2019-07-13)

* Added user prompt if the project has duplicates (user needs to pick desired project) 
* Fixed freezing bug when fetching files when samples is empty
* Now handles errors manually in main (with colored output)
* Added package into in Cargo.toml

# 0.1.1  (2019-06-12)

* Added support for Undetermined files with `-U` flag

# 0.1.0  (2019-06-11)

* First release
