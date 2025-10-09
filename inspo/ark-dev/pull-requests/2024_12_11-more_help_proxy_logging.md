# Add more logging for Help proxy errors

> <https://github.com/posit-dev/ark/pull/654>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3543 by adding some more logging for us to see more detail in what is going on

This PR moves `return HttpResponse::BadGateway().finish()` from where it is now to _after_ some logging we would do anyway, and then also adds logging for the error case (the status code plus just the whole body).

