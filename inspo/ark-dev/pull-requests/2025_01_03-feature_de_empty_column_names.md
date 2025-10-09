# Data Explorer: Return empty string in schema response instead of [, i] for empty column name

> <https://github.com/posit-dev/ark/pull/659>
> 
> * Author: @wesm
> * State: MERGED
> * Labels: 

Per discussion in https://github.com/posit-dev/positron/issues/3084, with code like

```
example <- data.frame(1:10, 1:10, 1:10)
names(example) <- c("", "age", "age ")
```

We have decided for now to return the column names as is, even if they are empty, and let the UI decide how to render them. With this change, we now see:

![image](https://github.com/user-attachments/assets/484a5841-9642-44f0-ad63-e1e34ad3d869)

