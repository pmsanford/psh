# psh

This is an experimental shell, partly inspired by [this blog post](https://arcan-fe.com/2022/10/15/whipping-up-a-new-shell-lashcat9/) about a much more ambitious shell. Being experimental, this is not suitable for day to day use.

Features:
- Aliases, accessed and created by the `alias` command.
- History and hints, provided by rustyline, accessed by pressing up at the command line (for history) and pressing right (for accepting hints)
- Cross-shell environment variable access, provided by the `diffenv` and `copyenv` commands.
- Cross-shell status information, provided by the `pshl` command.
- Remotely setting environment variables for the parent shell of other scripts and programs using the `setenv` utility (which allows me to avoid creating a scripting language - you can just write your pshrc in bash and call `setenv` to set variables in the parent environment)

Here's an example of these being used together: You've got a long-running process you don't want to kill, but it would be a pain to recreate parts of that environment to run another process. Or maybe you've had a process running for a long time and can't remember exactly how you configured its environment.

![image](https://user-images.githubusercontent.com/1696007/199389005-e065c165-fe8d-4367-94c4-403da2db3617.png)
