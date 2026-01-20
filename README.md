# ALACS - Associative and Linear Arrays of Commented Strings

+ [**MANIFESTO.md**](MANIFESTO.md) = an informal English description of the ALACS format.
+ [**ORIGINS.md**](ORIGINS.md) = the story of the spark that led to this project.


# what if keys gotta have newlines?

use the technique `jq` implements with `to_entries` and `from_entries` functions.
haven't tested this yet, but AI suggests this can be done transparently in pydantic:

```python
from pydantic import BaseModel, model_serializer, model_validator

class DynamicUser(BaseModel):
    id: int
    username: str

    @model_validator(mode='before')
    @classmethod
    def transform_from_entries(cls, data: Any) -> Any:
        # Check if the input is a list (the entries format)
        if isinstance(data, list):
            return {item["key"]: item["value"] for item in data}
        return data

    @model_serializer
    def to_entries_format(self):
        return [{"key": k, "value": v} for k, v in self]

entries_data = [{"key": "id", "value": 1}, {"key": "username", "value": "batman"}]
user = DynamicUser.model_validate(entries_data)
print(user.username) # Output: batman
```
