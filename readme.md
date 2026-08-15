Get characteristic response should return Vec<KnownCharacteristic<T>>
    - Where knowncharacteristic holds characteristic and T is the descriptor type

Descriptor type should impl some Trait that has associated type for Read value, expected write value
write to device