# DataViewer handles data frames with no names

> <https://github.com/posit-dev/ark/pull/13>
> 
> * Author: @romainfrancois
> * State: MERGED
> * Labels: 

addresses https://github.com/rstudio/positron/issues/640

Considering some cases with data frame and matrix columns: 

```r
.ps.view_data_frame(vctrs::new_data_frame(list(1, 2)))
.ps.view_data_frame(vctrs::new_data_frame(list(1, b = 2)))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, b = 2)))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, b = vctrs::new_data_frame(list(1, 2)))))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, b = vctrs::new_data_frame(list(1, b = 2)))))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, vctrs::new_data_frame(list(1, b = 2)))))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, vctrs::new_data_frame(list(1, 2)))))

.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, matrix(1:2, ncol = 2))))

mat <- matrix(1:2, ncol = 2)
colnames(mat) <- c("d", "e")
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, m = mat)))
.ps.view_data_frame(vctrs::new_data_frame(list(a = 1, mat)))
```

<img width="669" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/9d99beeb-1879-457b-b00c-ed1198f5d15b">

<img width="712" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/e944e2eb-5858-4c10-b490-cf184a931787">

<img width="745" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/68ddeede-ac14-4b22-815b-c4d7698e467c">

<img width="928" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/77597929-0392-4759-9eea-5531c1e5b702">

<img width="959" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/db0e82ee-37f5-4c9c-82b6-93f60c7f8db7">

<img width="935" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/4ab9723e-5343-4d27-af8a-74e742a088df">

<img width="909" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/c9916946-a970-49db-b2bd-0be46a8fb644">

<img width="826" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/46819542-faf0-4514-9358-d24d1226b541">

<img width="766" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/7f8d1674-72fc-42d9-9394-22b445bf91ed">

<img width="728" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/c373b0aa-2d0b-4047-9750-dac47645f0e8">



## @romainfrancois at 2023-05-30T15:33:40Z

Merging now before #11 but might need to follow up