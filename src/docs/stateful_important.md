Emit an important message to the end user.

When something significant happens but is not inherently negative, you can use an important
message. For example, if a buildpack detects that the operating system or architecture has
changed since the last build, it might not be a problem, but if something goes wrong, the
user should know about it.

Important messages should be used sparingly and only for things the user should be aware of
but not necessarily act on. If the message is actionable, consider using a
[`Print::warning`] instead.
