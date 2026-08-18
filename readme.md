Finish validate write data logic
    - git push
try refactor to make knowndecriptors a struct that has a field that is T: Foo, and a field that is known descriptor type enum
    - impl Foo for KnownDescriptor<T>