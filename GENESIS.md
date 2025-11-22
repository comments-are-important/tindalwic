
I was working on a project automating updates to human edited YAML files. It seemed
super easy: any YAML module can do that in only a few lines of code. So I wrote it up
with [PyYAML](https://github.com/yaml/pyyaml) and it worked... until I noticed that the
comments I had left in my files were disappearing. D'oh.

A quick web search led me to [ruamel.yaml](https://yaml.dev/doc/ruamel.yaml/)'s "round
trip" parser. It was easy to switch, and it preserved comments... as long as only
trivial modifications were attempted. I dug into the code a bit to see if I could coax
the behavior I wanted out of it, but it looked like that would be difficult.

More web searching led me to [TOML Kit](https://tomlkit.readthedocs.io/). To be honest,
I dislike TOML, but not as much as reinventing the wheel. Again it was relatively easy
to switch... but out of the box this still was not what I was looking for. Even though
the API for working with comments seemed better, it was hard to write code that made
complex updates.

At this point I took a step back to consider why none of these popular libraries was
making me happy. Eventually I concluded that the library APIs are hamstrung because the
specs all say to ignore the comments. To human readers, comments have subjects: they
talk about some other thing within the file. This fact needs to be part of the spec
before library authors can provide APIs that allow algorithms to fully collaborate with
humans in maintaining the files.

Linking comments to their subjects is not a new idea at all. JavaDoc and similar tools
are widely used, so people already know how this works - at least for those particular
types of comments. I have to admit that it feels a bit strange and onerous to extend
this to every single comment, but I don't see any other way to accomplish the goals.
This project has a small grammar, likely smaller than JSON or TOML, so that will
probably help alleviate those feelings.

<!-- mention Concrete Syntax Trees or not? both ruamel and tomlkit are CST builders.
if I could find one that treated comments better I would want to cite it. -->

I feel trepidation about trying to inflict yet another standard on the world,
particularly in this crowded niche. I agree with the authors of YAML, TOML, etc. that
excluding comments from JSON make it unfriendly to humans. But if you acknowledge the
importance of comments, then why not make them full-fledged first-class citizens?
