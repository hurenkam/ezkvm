## Contributing

As of now I'm the only active user (that I'm aware of) of this tool, and
so development is very much aimed at the features that i use myself on a
daily basis, since that is what makes sense to me, and that is what i can
easily and regularly test.

When you find this tool useful, then feel free to use it, as per the license
terms. When you use it, you may run into missing features, or perhaps discover
bugs or other problems. Please report these in the issues list.
It would be even better if you can attach a patch that addresses the issue.

### Creating an issue

For evaluating an issue, it is often essential to put some basic info
to aid triage:

1) Version of ezkvm that the problem occurred in. Currently there are no
   released versions yet, so please refer to the git commit (hash) that you
   noticed the behavior on. Also provide versions of qemu, and swtpm.
2) OS (and version) on which you are trying to run ezkvm.
3) Hardware (and versions) that you are running ezkvm on, especially if you
   are passing hardware to the VM.
4) The ezkvm config file that you use to startup the vm

### Providing a patch

Please have your patch refer to an issue which it intends to address,
create an issue if one does not exist yet.
Chop big changes into smaller (non breaking) changesets such that they each
address a small aspect. That makes the changes easier to review.

### Note:

Unless explicitly mentioned otherwise, I will assume that any contribution
follows the same licensing terms as the existing code. Also I will assume
that you are ok if i should decide to change these licensing terms, to another
open source license that may or may not be entirely compatible with the
current one.
