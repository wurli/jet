# Primitive functionality to display comment sections in outlines

> <https://github.com/posit-dev/ark/pull/571>
> 
> * Author: @kv9898
> * State: MERGED
> * Labels: 

Attempt to (partly) fix https://github.com/posit-dev/positron/issues/3822

![image](https://github.com/user-attachments/assets/066589b4-46be-44a9-b21e-0ed61b821117)

I've managed to make comments which are in the shape of "## Title ####/----/====" appear in the outline with the changes in this pull request, but I struggled to make them appear in a hierarchical form/foldable.

## @kv9898 at 2024-10-07T14:59:57Z

Hi @lionel-, thank you so much for the suggestions!! I have removed the line as per your suggestion and reformatted the code using +nightly. However, I'm really inexperienced with Rust (this is actually my first time writing Rust and I extensively used ChatGPT for this). 

I'm a bit struggling with the other suggestions. I'm not sure where do add the tests and how to add the the folding functionality. I have allowed edits by maintainers and would really appreciate if you can help/guide me through this because I personally really want this outline/folding functionality to work in Positron :)

## @lionel- at 2024-10-09T09:57:47Z

@kv9898 No worries, I've added a couple tests, feel free to add more from these examples.

## @kv9898 at 2024-10-10T02:02:32Z

@lionel-  Thank you so much for helping me out!  I'm pretty happy with how it looks now!

## @lionel- at 2024-10-14T11:39:29Z

Thanks a lot @kv9898!