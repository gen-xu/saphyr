use saphyr::{Map, Yaml, YamlEmitter};

#[test]
fn test_mapvec_legal() {
    // Emitting a `map<map<seq<_>>, _>` should result in legal yaml that
    // we can parse.

    let key = vec![Yaml::integer(1), Yaml::integer(2), Yaml::integer(3)];

    let mut keymap = Map::new();
    keymap.insert(Yaml::string("key".into()), Yaml::sequence(key));

    let val = vec![Yaml::integer(4), Yaml::integer(5), Yaml::integer(6)];

    let mut map = Map::new();
    map.insert(Yaml::map(keymap), Yaml::sequence(val));

    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&Yaml::map(map)).unwrap();
    }

    // At this point, we are tempted to naively render like this:
    //
    //  ```yaml
    //  ---
    //  {key:
    //      - 1
    //      - 2
    //      - 3}:
    //    - 4
    //    - 5
    //    - 6
    //  ```
    //
    // However, this doesn't work, because the key sequence [1, 2, 3] is
    // rendered in block mode, which is not legal (as far as I can tell)
    // inside the flow mode of the key. We need to either fully render
    // everything that's in a key in flow mode (which may make for some
    // long lines), or use the explicit map identifier '?':
    //
    //  ```yaml
    //  ---
    //  ?
    //    key:
    //      - 1
    //      - 2
    //      - 3
    //  :
    //    - 4
    //    - 5
    //    - 6
    //  ```

    Yaml::load_from_str(&out_str).unwrap();
}
