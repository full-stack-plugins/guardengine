import copy, json, pathlib, unittest
from jsonschema import Draft202012Validator
ROOT=pathlib.Path(__file__).resolve().parents[2]
class EnvelopeSchema(unittest.TestCase):
 def test_closed_schema_vectors(self):
  schema=json.loads((ROOT/'schemas/integration/v1alpha1/guard-run-envelope.schema.json').read_text())
  Draft202012Validator.check_schema(schema);validator=Draft202012Validator(schema)
  for path in sorted((ROOT/'tests/fixtures/integration-envelope').glob('*.json')):
   value=json.loads(path.read_text()); expected=not path.name.startswith('invalid-')
   self.assertEqual(validator.is_valid(value),expected,path.name)
  value=json.loads((ROOT/'tests/fixtures/integration-envelope/valid-native.json').read_text())
  for key in value:
   bad=copy.deepcopy(value);del bad[key];self.assertFalse(validator.is_valid(bad),key)
if __name__=='__main__':unittest.main()
