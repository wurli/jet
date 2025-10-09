# Fix regression in region detection

> <https://github.com/posit-dev/ark/pull/857>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Finishes addressing https://github.com/posit-dev/positron/issues/8059 (see https://github.com/posit-dev/ark/pull/842 for first part).

I mistakenly thought that VSCode regions were spec'd as:

```
*comment_opener* #region
```

I.e. you need `// #region` in Javascript or C. However I now see that for Python, which shares with R the `#` symbol as comment opener, it's actually:

```
*comment_opener*[optional_space]region
```

See https://github.com/microsoft/vscode/blob/d6d5034ff685d6aab2c1f226fef288455caa7a14/extensions/python/language-configuration.json#L45-L48

So I've just copied over their regexes for detection of regions.

