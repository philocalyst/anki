this is a specification for anki card decks that is designed to be used for collaboration and the sharing of knowledge in the flashcard format. There is no reason why it cannot be used outside of anki, but was created around their, very abstract and open, model. It could be considered a successor to crowd anki, which while is very useful and beloved, I feel makes certain tradeoffs that are counter-intuitive to a successful community, and usage in the plain-text world. This format is designed around a use-case that is primarily plain-text, and is focused on ergonomics and integration with existing tools, segmenting content around natural boundaries.

It can be consumed in one of two ways.

One, for simplicity, where the format relies heavily on shared understanding of certain conventions to scale. These conventions must be followed in order to leverage the format to its fullest potential, and are in place to try and maintain simplicity while keeping a continuity. If these conventions are followed, a place is earned on the registry.

Two, for ease of use, where the format abandons some its simplicity for ease, and the most annoying conventions can be discarded. This primarily means one thing: you must annotate each block with a UUID. This means that assuming the UUID is never modified, you can ignore the convention of moving blocks around.

## The Conventions
- Any public deck or collection repository must included the topic "flashcards". This is for discoverability and inclusion into the registry.
- New notes can be added, and old notes can be deleted, but there cannot be a reordering without a .changes file that describes the position change that occured. Example: If you moved the 25th block in a file to position 16, you would record it as 25->16.
- Any card deleted or added warrants a minor version bump.
- Any field deleted or added warrants a major version bump.
- Any field name modified warrants a major version bump.


## Categories of Change
### Addition
- This is an arbitrary number of added blocks


## The Expected Structure
- The entrypoint to the parsing is any folder with a .deck extension. All operations happen from the purview of an entrant into that folder.
- Inside the folder, all subdirectories are assumed to be note models. These can be named whichever your filesystem supports, with the absence of "Assets", which, if found, is the only exception.
- Assets folders are where all media related to the deck should be stored.
- Any file with a .flash extension will be parsed as a flashcard file.


All note model folders contain:
- A config.toml file that determines options that define the model
- Any number of hbs (mustache) files, which represent the templates that model holds. Their naming scheme is as follows: NAME_OF_TEMPLATE+FRONT/BACK<.browser>.hbs
  - The .browser is included BEFORE the .hbs if it is a browser variation of a template.
  - The +Front or +Back is a determiner if it is for the front or back of the card.
- An optional pre.tex and/or post.tex, which are latex information to be included at the beginning or end of the model.
- A style.css file containing the styles for the model.
