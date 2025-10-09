# data explorer: set edges to value of single bin

> <https://github.com/posit-dev/ark/pull/884>
> 
> * Author: @isabelizimm
> * State: MERGED
> * Labels: 

R half of https://github.com/posit-dev/positron/pull/8611
Addressing https://github.com/posit-dev/positron/issues/8095


```
View(data.frame(values = rep(10, 10)))
```

Upper and lower values should be equal on hover.

<img width="372" height="128" alt="Screenshot 2025-07-29 at 10 48 00 AM" src="https://github.com/user-attachments/assets/462b4629-9f5f-4606-bb69-c3eef7d4da1f" />


