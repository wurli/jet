# Language server allows recognize column's name

> <https://github.com/posit-dev/ark/issues/814>
> 
> * Author: @ntluong95
> * State: OPEN
> * Labels: 

I dont know how possible to implement this feature, as currently the LSP is able to display column name in code completion. What I want is that they have their own token, which allow for syntax highlighting feature. My expectation is to display column name with different color, so when I scan code I can quickly recognize them

Example code

```r
# create a sample data frame
my_data <- data.frame(
  id = 1:5,
  value = c(10, 20, 30, 40, 50)
)

my_data %>% 
  select(id, value) %>% 
  filter(value > 30) 
```
Expected behavior:
Color for `id` and `value` is displayed differently. 

![Image](https://github.com/user-attachments/assets/a69ac7a1-f0ad-42cd-9baa-5aca99d3be93)




