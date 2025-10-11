# Fix parameter documentation on R >=4.3.1

> <https://github.com/posit-dev/ark/pull/771>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/5302

Ah, welcome back to our old friend `R.css`!

I've explained the problem well here so I won't rehash it:
https://github.com/posit-dev/positron/issues/5302#issuecomment-2512657264

The solution is to just not "select" on `[style="vertical-align: top;"]` at all, since it no longer exists in R >=4.3.1, and to instead just select on `tr > td` while inside a `<table>` nested inside the `<h3>Arguments</h3>` header. Hopefully that is precise enough to not come up with false positives, and even if it does hit false positives it is unlikely that they work with the rest of the code that works on the selection itself. This has the benefit of also working in R < 4.3.1 when the extra styling was still there.

---

Here it is on R 4.2.1 (when `<tr style="vertical-align: top;"><td>` was still there, this works on `main`)

<img width="610" alt="Screenshot 2025-04-10 at 2 06 05 PM" src="https://github.com/user-attachments/assets/705ae369-1674-404e-a83b-f0cdd887de53" />

Here it is on R 4.4.3 (this only works with this PR)

<img width="609" alt="Screenshot 2025-04-10 at 2 06 23 PM" src="https://github.com/user-attachments/assets/ab657465-5a84-4518-bb6a-0b75d5aec2eb" />

---

One thing of note is that it looks like a monospaced font is being used for the help documentation now. That must be a fairly new VS Code change, because this image linked in the original issue did not use a monospaced font!

![image](https://github.com/user-attachments/assets/4f2aa423-37c9-4297-b05d-bcc0a5c63db2)


## @DavisVaughan at 2025-04-10T18:52:02Z

Oh interesting there truly is some difference between the font in the Console and the font in the Editor for parameter documentation

https://github.com/user-attachments/assets/55748160-9f91-4308-badb-765c4661b443

Interesting, in the Console it looks like it is trying to respect the `editor.fontFamily`, but in the Editor it does not (and probably should not? not sure.). Its easy to see if you set the editor font family to times new roman



<img width="608" alt="Screenshot 2025-04-10 at 2 55 34 PM" src="https://github.com/user-attachments/assets/3c54b7e3-0dbd-4e87-9d0a-ce387ffc5f55" />
<img width="569" alt="Screenshot 2025-04-10 at 2 55 44 PM" src="https://github.com/user-attachments/assets/e0c0510a-833d-4c5d-86a3-ae5a6b6b613c" />

