# TODO

* Check etags to ensure file downloaded correctly

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
