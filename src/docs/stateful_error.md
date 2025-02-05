Emit an error and end the build output.

When an unrecoverable situation is encountered, you can emit an error message to the user.
This associated function will consume the build output, so you may only emit one error per
build output.

An error message should describe what went wrong and why the buildpack cannot continue.
It is best practice to include debugging information in the error message. For example,
if a file is missing, consider showing the user the contents of the directory where the
file was expected to be and the full path of the file.

If you are confident about what action needs to be taken to fix the error, you should include
that in the error message. Do not write a generic suggestion like "try again later" unless
you are certain that the error is transient.

If you detect something problematic but not bad enough to halt buildpack execution, consider
using a [`Print::warning`] instead.
