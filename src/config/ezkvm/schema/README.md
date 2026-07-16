This directory contains only ezkvm schema related types.
Files in this directory shall not depend on types in runtime
Any conversion between runtime <--> ezkvm files will be specified
in files in the src/config/ezkvm/runtime directory to keep the
src/config/ezkvm/schema directory clean of such dependencies.
