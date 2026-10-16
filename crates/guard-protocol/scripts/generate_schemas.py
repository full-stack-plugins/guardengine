"""生成固定的 Draft 2020-12 结构 Schema；不把它冒充运行时契约编译器。"""
import argparse, hashlib, json, re
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
META = json.loads((ROOT / 'model_metadata.json').read_text(encoding='utf-8'))
REF = lambda name: {'$ref': '#/$defs/' + name}
MAX = 9007199254740991

def enum_wire(value, style='SCREAMING_SNAKE_CASE'):
    if style == 'lowercase': return value.lower()
    snake = re.sub(r'(?<!^)(?=[A-Z])', '_', value)
    return snake.lower() if style == 'snake_case' else snake.upper()

def schema_type(typ):
    if typ.startswith('Vec<'): return {'type':'array','maxItems':8192,'items':schema_type(typ[4:-1])}
    if typ.startswith('Option<'): return {'anyOf':[schema_type(typ[7:-1]),{'type':'null'}]}
    return {'bool':{'type':'boolean'},'u64':{'type':'integer','minimum':0,'maximum':MAX},
        'u32':{'type':'integer','minimum':0,'maximum':4294967295},
        'i32':{'type':'integer','minimum':-2147483648,'maximum':2147483647},
        'serde_json::Value':REF('JsonValue')}.get(typ,REF(typ))

def build():
    defs = {
        'Identifier':{'type':'string','minLength':1,'maxLength':4096,'pattern':r'^[^\u0000-\u001f\u007f-\u009f]+$', 'allOf':[{'pattern':r'[^\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]'}], 'x-semanticConstraint':'UTF-8 byte length <=4096; JSON Schema maxLength counts Unicode code points'},
        'Digest':{'type':'string','pattern':'^sha256:[0-9a-f]{64}$'},
        'ArtifactName':{'type':'string','minLength':1,'maxLength':255,'pattern':r'^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9_-])?$',
            'allOf':[{'not':{'pattern':r'\.\.'}},{'not':{'pattern':r'^(?:[cC][oO][nN]|[pP][rR][nN]|[aA][uU][xX]|[nN][uU][lL]|[cC][oO][mM][1-9]|[lL][pP][tT][1-9])(?:\..*)?$'}}],
            'description':'ASCII logical basename; not a filesystem path; rejects traversal and reserved device names'},
        'ArtifactMediaType':{'type':'string','minLength':3,'maxLength':255,'pattern':r'^[a-z0-9!#$&^_.+\-]+/[a-z0-9!#$&^_.+\-]+$',
            'description':'restricted lowercase type/subtype without parameters, whitespace or extra slash'},
        'GitOid':{'type':'string','pattern':'^(?:[0-9a-f]{40}|[0-9a-f]{64})$'},
        'SchemaVersion':{'const':'1.0.0'},
        'RunSequence':{'type':'string','pattern':'^(?:0|[1-9][0-9]{0,19})$','x-semanticConstraint':'must fit u64'},
        'UtcTimestamp':{'type':'string','format':'date-time','pattern':r'^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]{1,9})?Z$','x-semanticConstraint':'valid calendar date; no leap-second or year zero'},
        'GitPath':{'type':'object','additionalProperties':False,'required':['encoding','bytes'],
            'properties':{'encoding':{'const':'base64'},'bytes':{'type':'string','minLength':4,'maxLength':5464,
                'contentEncoding':'base64','pattern':r'^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$'}},
            'x-semanticConstraint':'canonical base64 of nonempty relative raw Git path; no NUL, empty, dot or dot-dot segment'},
        'Extensions':{'type':'object','maxProperties':2048,'patternProperties':{r'^[a-z][a-z0-9.-]*:[A-Za-z0-9._-]+$':REF('JsonValue')},'additionalProperties':False,'propertyNames':{'maxLength':256}},
        'JsonValue':{'anyOf':[{'type':'null'},{'type':'boolean'},
            {'type':'number'},{'type':'string','maxLength':65536,'x-semanticConstraint':'UTF-8 byte length <=65536; JSON Schema maxLength counts Unicode code points'},
            {'type':'array','maxItems':8192,'items':REF('JsonValue')},
            {'type':'object','maxProperties':2048,'additionalProperties':REF('JsonValue')}],
            'x-inputConstraint':'strict raw parser rejects duplicates, non-finite numbers, depth > 64 or input > 2 MiB; protocol integer identities must be safe or strings'}
    }
    for filename,(name,variants,doc) in META['enums'].items():
        style = {'domain_owner':'lowercase','object_format':'lowercase','subject_kind':'snake_case','release_artifact_kind':'snake_case'}.get(filename,'SCREAMING_SNAKE_CASE')
        defs[name]={'type':'string','enum':[enum_wire(v,style) for v in variants],'description':doc}
    for name,(doc,fields) in META['structs'].items():
        defs[name]={'type':'object','additionalProperties':False,'description':doc,
            'properties':{field:schema_type(typ) for field,typ in fields},
            'required':[field for field,typ in fields if not typ.startswith('Option<')]}
    for name in ['timeoutMs','maxOutputBytes','maxConcurrency']:
        defs['ExecutionBudget']['properties'][name]['minimum']=1
    for name in ['startLine','startColumn','endLine','endColumn']:
        defs['SourceSpan']['properties'][name]['minimum']=1
    defs['TargetSelection']['x-semanticConstraint']='approved scope only; optional protectedDigest must equal bound candidate inputDigest; duplicate repository/path/selector selections rejected'
    defs['ContractObligation']['x-semanticConstraint']='preserve approved obligation/required/evidence/target selection sets when binding actual candidate inputs'
    defs['GitSubjectSnapshot']['allOf']=[]
    for kind,required in {
        'worktree':['worktreeId','diffDigest'],'index':['worktreeId','head','tree','diffDigest'],
        'commit':['head','tree'],'merge_candidate':['base','head','tree','targetRef','expectedTargetOid','diffDigest']}.items():
        defs['GitSubjectSnapshot']['allOf'].append({'if':{'properties':{'kind':{'const':kind}},'required':['kind']},
            'then':{'required':required,'properties':{k:{'not':{'type':'null'}} for k in required}}})
    defs['ReleaseSourceIdentity']['x-semanticConstraint']='commit/tree full OID lengths must match objectFormat; existence and Git relationship require independent capture'
    defs['ReleaseSourceIdentity']['allOf']=[
        {'if':{'properties':{'objectFormat':{'const':algorithm}},'required':['objectFormat']},
         'then':{'properties':{field:{'pattern':f'^[0-9a-f]{{{length}}}$'} for field in ['commit','tree']}}}
        for algorithm,length in [('sha1',40),('sha256',64)]]
    for name, variants in META.get('unions', {}).items():
        defs[name]={'oneOf':[REF(variant) for variant in variants],
            'description':'Closed disjoint candidate forms; release artifacts cannot use Git worktree/commit fields'}
    defs['FactEnvelope']['oneOf']=[]
    for kind,key in [('SYMBOL','symbol'),('EDGE','edge'),('SOURCE_SPAN','sourceSpan')]:
        defs['FactEnvelope']['oneOf'].append({'properties':{'factType':{'const':kind},key:{'not':{'type':'null'}},
            **{other:{'type':'null'} for other in ['symbol','edge','sourceSpan'] if other!=key}},'required':[key]})
    for name in META['documents']:
        if name in META.get('unions', {}):
            branches=[]
            for variant in META['unions'][name]:
                obj=json.loads(json.dumps(defs[variant])); obj['properties']['documentType']={'const':name}; obj['required'].append('documentType')
                defs[variant+'Document']=obj; branches.append(REF(variant+'Document'))
            defs[name+'Document']={'oneOf':branches,'required':['schemaVersion','kind','repositoryId','inputManifestDigest','extensions','documentType']}
            continue
        obj=json.loads(json.dumps(defs[name])); obj['properties']['documentType']={'const':name}; obj['required'].append('documentType')
        defs[name+'Document']=obj
    return {'$schema':'https://json-schema.org/draft/2020-12/schema',
        '$id':'https://schemas.partme.ai/guard/v1/engineering.schema.json',
        'title':'Engineering Guard interchange v1 (RFC8785 / IEEE754)',
        'description':'Parsing validates structure only; trust, actual objects, signatures and authorization require independent verification.',
        'oneOf':[REF(n+'Document') for n in META['documents']], '$defs':defs}

def outputs():
    result={'schemas/engineering.schema.json':build()}
    for name in META['documents']:
        result['schemas/'+re.sub(r'(?<!^)(?=[A-Z])','-',name).lower()+'.schema.json']={
            '$schema':'https://json-schema.org/draft/2020-12/schema',
            '$id':'https://schemas.partme.ai/guard/v1/'+re.sub(r'(?<!^)(?=[A-Z])','-',name).lower()+'.schema.json',
            '$ref':'engineering.schema.json#/$defs/'+name+'Document'}
    return {p:json.dumps(v,ensure_ascii=False,indent=2)+'\n' for p,v in result.items()}

def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--check',action='store_true'); args=parser.parse_args()
    for name,text in outputs().items():
        path=ROOT/name
        if args.check:
            if not path.is_file() or path.read_text(encoding='utf-8')!=text: raise SystemExit('schema drift: '+name)
        else: path.write_text(text,encoding='utf-8',newline='\n')
    print('schema source alignment OK' if args.check else 'schemas generated')
if __name__=='__main__': main()
