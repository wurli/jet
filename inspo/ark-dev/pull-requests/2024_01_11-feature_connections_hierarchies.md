# Support hierarchy of objects in database connections

> <https://github.com/posit-dev/ark/pull/204>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Targets https://github.com/posit-dev/positron/pull/2042

This PR add's support for hierarchies of obejcts in database connections. It must be tested with the frontend changes within https://github.com/posit-dev/positron/pull/2049.

This is a possible testing snippet:

```
tmp <- tempfile()
dir.create(tmp)
dbplyr::nycflights13_sqlite(path = tmp)
con <- connections::connection_open(RSQLite::SQLite(), file.path(tmp, "nycflights13.sqlite"))
```

https://github.com/posit-dev/amalthea/assets/4706822/a121ef69-82c0-4ab9-8979-dbe006ae9756




## @dfalbel at 2024-01-12T14:18:43Z

Thank you!