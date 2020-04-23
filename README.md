# application-operator

application-operator relies on a few things being explicitly set

* `CONFIG_VERSION` environment variable - typically `git rev-parse HEAD` or `git describe --tags` make
  good examples for this. It's expected that whatever automation maintains the operator keeps this value
  updated
* `--template job-template.yml` - this is the template for the job that application-operator creates
  when either configuration changes or an application changes.
* `--image image` docker image to pass to the job template


## Installation

* Install the Custom Resource Definition (see docs/crd.yaml or docs/crd-legacy.yaml).
  You'll need the legacy CRD for Kubernetes 1.15 and below.
* Install the operator (see docs/deploy.yaml)
* Set up applications by creating new Application resources (see docs/example.yaml)
* Add RBAC permissions for the ServiceAccount for the job (set `--service-account`
  to set the service account used by the job)

You will need a container that can do deployments. The example job template assumes
that the container can run `/bin/deploy application environment version` and that will
do the trick. The underlying mechanism is up to you, but I'll provide an example
container that uses ansible-runner to do the work. You can pass this as `--image`
(e.g. `--image application-operator/ansible-config`)

This container should either mount the configuration (e.g. from a git-sync sidecar
container) or be built with the configuration included. My preference is to build
the container with the configuration inside, tagged with the version of the code.

## Job Names

Job names are derived from the application name (as per `spec.application` in the resource
definition), the configuration version (from `CONFIG_VERSION`) and the application version
(`spec.version`). As Jobs have a maximum length of 63, we shorten the versions to 16
and impose a maximum length of 29 on the application name (to leave two characters for the
separators).

As Jobs only run once per job name, this means that the first 16 characters of versions
must be distinct across different versions - easy with a commit hash or `git describe --long`
but if you're `doing this-is-a-very-long-tag-prefix-$(git describe --long)` it won't work as well.
