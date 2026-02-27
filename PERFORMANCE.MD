# Big Ones

- to_web_mercator: ~5% -> Cache mercator pos on Node    (Brought this down to 2.5% with more efficient math)
- get_node_pos: ~7% -> Cache mercator pos of nodes on Way

# Maybe unavoidable:

- get_style Hashmap lookup: ~7% -> Store default node style separately on styles and add flag unstyled on node?

# Actually ok

- tesselation: 2.5%
- draw_element: 7%