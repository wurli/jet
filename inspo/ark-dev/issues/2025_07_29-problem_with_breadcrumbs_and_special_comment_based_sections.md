# Problem with breadcrumbs and special-comment based sections

> <https://github.com/posit-dev/ark/issues/846>
>
> * Author: @juliasilge
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwDw", name = "enhancement", description = "New feature or request", color = "C5DEF5")

_Originally posted by @SamT123 in https://github.com/posit-dev/positron/discussions/8205_:

Hi!

I think there may be a bug with breadcrumbs when using comment-defined sections in R (i.e. "# title ------") – but want to check here before opening an issue.

When not using comment-defined sections, `breadcrumbs.focus` starts with the "highlighting" focussed on the part of the code you are currently in, which is very useful for navigation:

i.e. if I do `breadcrumbs.focus` when my cursor is here, in `my_fun_2.2`:

<img width="646" alt="Screenshot 2025-06-19 at 16 36 33" src="https://github.com/user-attachments/assets/91c8dcb3-bb96-467f-8be9-d58864d2093d" />

breadcrumbs starts with my_fun_2.2 highlighted, and I can navigate from there with the arrow keys:

<img width="646" alt="Screenshot 2025-06-19 at 16 36 50" src="https://github.com/user-attachments/assets/d6ef965b-7c77-4ec5-8c27-3bab2d405dac" />

But, if there is are comment-defined sections above it, breadcrumbs seemingly doesn't know where I am in the code:

<img width="643" alt="Screenshot 2025-06-19 at 16 37 08" src="https://github.com/user-attachments/assets/0131e1d9-84c3-4110-bf32-f6fc1db72230" />


**EDIT:**

The hierarchy / structure of the document in breadcrumbs is correct, its just that the highlighting doesn't start in the right place. Here I've have expanded out the folded sections:

<img width="647" alt="Screenshot 2025-06-19 at 16 48 58" src="https://github.com/user-attachments/assets/07b824c2-dd3f-485e-8948-6830f241ce79" />


Positron Version: 2025.07.0 (Universal) build 134
Code - OSS Version: 1.100.0
Commit: 572a37b9b646af0b813dbe4d424bc9bfa1629646
Date: 2025-06-18T03:33:46.852Z
Electron: 34.5.1
Chromium: 132.0.6834.210
Node.js: 20.19.0
V8: 13.2.152.41-electron.0
OS: Darwin arm64 24.4.0

Any help much appreciated!
Sam



## @lionel- at 2025-06-23T09:08:45Z

_Possibly_ related: if you select the section in the breadcrumbs popup, only the comment is highlighted, instead of the whole section. This does not happen with functions which are highlighted as a whole.

https://github.com/user-attachments/assets/3651da7b-57dc-464b-ba9a-55c511b1d6d6
