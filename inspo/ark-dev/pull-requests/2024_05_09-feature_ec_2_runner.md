# Add compatibility with EC2 runner

> <https://github.com/posit-dev/ark/pull/348>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change makes it possible to compile Ark in the EC2 environment.

- Remove revive-agent job, which is unnecessary on EC2
- Use system Homebrew paths

Companion to https://github.com/posit-dev/positron/pull/3081 (more notes there)

