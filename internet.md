So, how does this go from this format to something you might see on the interwebes.


Well I was thinking of taking an approach that, in the late-stages, can rival quizlet.

The structure, and what makes this feasible is the raw fact that I DONT have to worry about creating a "flashcard experience" I can let Anki handle that for me.

All I'm really doing is creating a gallery. Where the hosting might not even be done by me (largely).

The flow is:
- Someone creates a deck in this new format, whatever it might be called.
- They publish it/version it behind github or gitlab
- They name it and tag it appropriately.
- My bots "pick it up" and immeidately parse and present it on the website.
- Then any interested fellow can go through the cards, see the presentation, and, if it's something they would like to keep, can easily import it to their Anki collection, or (if anki is nice to me), can begin studying right then, on the web.


## The future
- A complete editor for creating the cards, as I would like regular people who don't know how to code to go and make their own, and build on what came before. Because you wouldn't be able to just edit and anki deck and rexport it? Maybe you could???
- Export to all of the major formats, import from all the major formats
- An init command to fill in the regular information for you
- Supporting lua modules where people can define migrations
- Buildng top of it experience similar to anki, but for more of the last-moment consumer. Anki is great but comittal, and I think a lot of people exercising the flow, having gotten where they are without even paying, would be reulcuctant to go that far. If we could make some kind of games, or blitz modes, anything of that regard, it could get more fun.
- Related to the last point, I want to give users accounts and make it stateful one day. Obviously this goes against a lot of my ethos when taken at face value, so I want to be explict about keeping other avenues of interaction up. I might even add an email portal.
