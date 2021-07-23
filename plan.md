# Eldritch #
    
## TODO ##
 * Change movement mode to use the edges of the screen.
 * Have 2 renders of the world, one using an "Eldritch" flag and one using "normal"
   * "Normal" renders first and "Eldritch renders on top
 * Modify the Eldritch shader depending on player stats
 
## GamePlay ##
 * Choose your character
 * Get your story prompt
 * Spawn into the world
 * Investigate for clues and collect evidence
   - Interrogate citizens
   - Search for supernatural signs
   - Try to find the next destination
   - Avoid cultists
   - Avoid creatures
   - Collect the clues and artifacts you find in various ways
 * Maybe there's a "hub" where you need certain artifacts and/or experience to enter different "dimensions"
 * Some Combat?
 * When the clues have been solved, the players have the chance to stop the event.
 * Any low stat causes Insanity to rise faster
 * Insanity is visual
   - There is another render of the world that warps at the edges
   - Words are harder to read when insane, and start to turn to glyphs
   - People in the corner of your eye look monstrous
   - You can find otherworldly clues easier
   - Enemies are more easily drawn to you
 
## Player Actions ##
 * Walk
 * Run (Consumes stamina)
 * Turn using screen edges
 * Sacrifice sanity to find clues and pierce the veil
 * Mouse interactions
   - Inspect (Visual)
   - Inspect (Physical)
   - Take
   - Talk
   - Make Note (In Journal)
 * Combat (Pseudo turn-based)
   - Actions are limited by stats
   - Attacking makes you unable to run for a time
   - Use environmental hazards
     * Push off roof
     * Set traps ahead of time
     * Have teammates hit enemy from behind
     
## Engine Features ##
 * ~~Run on WebGL~~
 * Multiple light sources
   - 
 * First Person movement
   - Edge of screen turning
 * UI framework
   - This is a really big ask
   - egui?
 * Texture Sampling
   - Edge of screen turning
 * PBR Support?
 * Blender File support

## JavaScript <--> Rust ##
 * Load files from Server
 * Networking Sockets

