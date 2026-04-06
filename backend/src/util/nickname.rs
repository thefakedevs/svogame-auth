use rand::Rng;
use rand::seq::IndexedRandom;

const ADJECTIVES: [&str; 256] = [
    "Able", "Acid", "Aged", "Airy", "Ajar", "Akin", "All", "Any", "Apt", "Arch",
    "Arid", "Avid", "Back", "Bad", "Bald", "Bare", "Base", "Bent", "Best", "Big",
    "Blue", "Bold", "Born", "Both", "Busy", "Calm", "Chic", "Cold", "Cool", "Coy",
    "Cute", "Cyan", "Damp", "Dank", "Dark", "Dead", "Deaf", "Dear", "Deep", "Deft",
    "Dim", "Dire", "Done", "Down", "Drab", "Dry", "Dual", "Due", "Dull", "Dumb",
    "Each", "East", "Easy", "Else", "Epic", "Even", "Evil", "Fair", "Fake", "Far",
    "Fast", "Fat", "Few", "Fine", "Firm", "Fit", "Five", "Flat", "Fond", "Foul",
    "Four", "Free", "Full", "Game", "Glad", "Glib", "Glum", "Gold", "Gone", "Good",
    "Gray", "Grey", "Grim", "Hale", "Half", "Hard", "Held", "High", "Hind", "Hip",
    "Holy", "Hot", "Huge", "Hurt", "Iced", "Icy", "Idle", "Iffy", "Ill", "Inky",
    "Just", "Keen", "Kept", "Key", "Kind", "Knit", "Lacy", "Lame", "Lank", "Last",
    "Late", "Lax", "Lazy", "Lean", "Left", "Less", "Lewd", "Like", "Limp", "Lit",
    "Lite", "Live", "Lone", "Long", "Lost", "Loud", "Low", "Lush", "Mad", "Made",
    "Main", "Male", "Many", "Mass", "Mean", "Meek", "Mere", "Mid", "Mild", "Mini",
    "Mint", "Mock", "Moot", "More", "Most", "Much", "Mute", "Navy", "Near", "Neat",
    "Neon", "New", "Next", "Nice", "Nigh", "Nil", "Nine", "No", "None", "Nude",
    "Null", "Numb", "Odd", "Off", "Oily", "Okay", "Old", "One", "Only", "Open",
    "Oral", "Our", "Out", "Oval", "Own", "Paid", "Pale", "Past", "Pert", "Pink",
    "Plus", "Poor", "Posh", "Prim", "Punk", "Puny", "Pure", "Rare", "Rash", "Raw",
    "Real", "Rear", "Red", "Rich", "Rife", "Ripe", "Rosy", "Rude", "Sad", "Safe",
    "Sage", "Same", "Sane", "Sexy", "Shot", "Shut", "Shy", "Sick", "Side", "Six",
    "Slim", "Slow", "Sly", "Smug", "Snug", "Soft", "Sole", "Solo", "Some", "Sore",
    "Sour", "Sown", "Spry", "Stag", "Such", "Sure", "Tall", "Tame", "Tan", "Tart",
    "Taut", "Teal", "Teen", "Ten", "That", "Then", "Thin", "This", "Tidy", "Tiny",
    "Top", "Torn", "Trim", "True", "Twin", "Two", "Ugly", "Used", "Vain", "Vast",
    "Very", "Vile", "Void", "Wan", "Warm", "Wary"
];

const NOUNS: [&str; 305] = [
    "Acid", "Act", "Age", "Aid", "Aim", "Air", "Ant", "Arc", "Arm",
    "Art", "Ash", "Atom", "Aunt", "Auto", "Axe", "Babe", "Baby", "Back", "Bag",
    "Bail", "Bait", "Ball", "Band", "Bang", "Bank", "Bar", "Barn", "Base", "Bass",
    "Bat", "Bath", "Bead", "Beak", "Beam", "Bean", "Bear", "Beat", "Bed", "Bee",
    "Beef", "Beer", "Bell", "Belt", "Bet", "Bias", "Bid", "Bike", "Bill", "Bin",
    "Bird", "Bit", "Blob", "Blog", "Boat", "Body", "Bog", "Boil", "Bolt", "Bomb",
    "Bond", "Bone", "Book", "Boom", "Boot", "Boss", "Bowl", "Box", "Boy", "Bra",
    "Bran", "Bug", "Bulb", "Bull", "Bump", "Bun", "Bus", "Bush", "Byte", "Cab",
    "Cage", "Cake", "Call", "Camp", "Can", "Cap", "Car", "Card", "Care", "Cart",
    "Case", "Cash", "Cast", "Cat", "Cave", "Cell", "Cent", "Chef", "Chin", "Chip",
    "City", "Clan", "Clay", "Club", "Clue", "Coal", "Coat", "Code", "Coil", "Coin",
    "Cold", "Comb", "Cone", "Cook", "Cop", "Cord", "Cork", "Corn", "Cost", "Cot",
    "Cow", "Crab", "Crew", "Crow", "Cube", "Cup", "Dad", "Dam", "Date", "Dawn",
    "Day", "Deal", "Debt", "Deck", "Deed", "Deer", "Dent", "Desk", "Dial", "Dice",
    "Diet", "Dirt", "Disc", "Dish", "Disk", "Dock", "Dog", "Doll", "Dome", "Door",
    "Dot", "Dove", "Drag", "Drop", "Drug", "Drum", "Duck", "Dust", "Duty", "Ear",
    "Ease", "East", "Edge", "Egg", "Ego", "Elf", "Elm", "End", "Envy", "Era",
    "Eve", "Exit", "Eye", "Face", "Fact", "Fad", "Fail", "Fair", "Fall", "Fan",
    "Farm", "Fate", "Fault", "Fear", "Fee", "Feed", "Feet", "Film", "Fin", "Fire",
    "Fish", "Fist", "Flag", "Flaw", "Flea", "Flow", "Flu", "Fly", "Foam", "Fog",
    "Foil", "Folk", "Food", "Foot", "Fork", "Form", "Fort", "Fox", "Frog", "Fuel",
    "Fun", "Fund", "Fur", "Fuse", "Gag", "Gain", "Gala", "Gale", "Game", "Gap",
    "Gas", "Gate", "Gear", "Gem", "Gene", "Gift", "Gig", "Girl", "Glee", "Glow",
    "Glue", "Goal", "Goat", "God", "Gold", "Golf", "Grab", "Gram", "Grip", "Grit",
    "Gulf", "Gum", "Gun", "Guru", "Gym", "Hack", "Hair", "Hall", "Halo", "Hand",
    "Hare", "Harm", "Hat", "Hate", "Hawk", "Hay", "Head", "Heap", "Heat", "Heel",
    "Heir", "Hell", "Helm", "Help", "Herb", "Hero", "Hill", "Hint", "Hip", "Hole",
    "Home", "Hood", "Hook", "Hope", "Horn", "Hose", "Host", "Hour", "Hug", "Hull",
    "Hunt", "Hut", "Ice", "Idea", "Idol", "Inch", "Ink", "Inn", "Ion", "Iron",
    "Isle", "Item", "Jail", "Jam", "Jar", "Jaw", "Jazz", "Jeep", "Jet", "Job",
    "Joke", "Joy", "Jump", "Junk", "Jury", "Keys"
];

pub fn random_nickname() -> String {
    let mut rng = rand::rng();

    // 4 letter
    let adjective = ADJECTIVES.choose(&mut rng).unwrap();

    // 4 letter
    let noun = NOUNS.choose(&mut rng).unwrap();

    // 6-digit number
    let number = rng.random_range(100000..999999);

    format!("{}_{}_{}", adjective, noun, number)
}
