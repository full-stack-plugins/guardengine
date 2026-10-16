"""Run real isolated Cargo/Maven fixtures through execution, trace and stage evidence.

Requires explicit toolchain paths. Executes only generated fixtures, never a business repo.
Evidence and keys remain in a new directory under --work-dir for inspection.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) if isinstance(value, dict) else value, encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--work-dir', type=Path, required=True)
    parser.add_argument('--cargo-executable', type=Path, required=True)
    parser.add_argument('--maven-executable', type=Path, required=True)
    args = parser.parse_args()
    base = args.work_dir.resolve(strict=True) / ('execution-loop-' + uuid.uuid4().hex)
    base.mkdir()

    def call(tool, *argv, code=0):
        binary = args.bin_dir.resolve() / (tool + ('.exe' if os.name == 'nt' else ''))
        result = subprocess.run([str(binary), *map(str, argv)], capture_output=True,
                                encoding='utf-8', timeout=300)
        assert result.returncode == code, (tool, argv, result.returncode,
                                           result.stdout[-6000:], result.stderr[-6000:])
        return json.loads(result.stdout)

    private, public = base / 'private.key', base / 'public.key'
    call('testguard', 'keygen', '--private', private, '--public', public)
    results = []
    for runner, executable, test_id in [('cargo', args.cargo_executable, 'tests::accepts'),
                                        ('maven', args.maven_executable, 'ExampleTest#accepts')]:
        project = base / (runner + '-project')
        project.mkdir()
        if runner == 'cargo':
            write(project / 'Cargo.toml', '[package]\nname="guard_loop"\nversion="0.1.0"\nedition="2021"\n')
            source = project / 'src/lib.rs'
            good = 'pub fn accepts() -> bool { true }\n#[cfg(test)] mod tests { #[test] fn accepts() { assert!(super::accepts()); } }\n'
            write(source, good)
            subprocess.run([str(executable), 'generate-lockfile', '--offline'],
                           cwd=project, check=True, capture_output=True)
            marker = 'pub fn accepts'
        else:
            write(project / 'pom.xml', '<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>guard.fixture</groupId><artifactId>loop</artifactId><version>1</version><properties><maven.compiler.release>17</maven.compiler.release></properties><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency></dependencies><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><version>3.13.0</version></plugin><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-surefire-plugin</artifactId><version>3.2.5</version></plugin></plugins></build></project>')
            source = project / 'src/test/java/ExampleTest.java'
            good = 'import org.junit.Test; import static org.junit.Assert.assertTrue; public class ExampleTest { @Test public void accepts() { assertTrue(true); } }\n'
            write(source, good)
            marker = 'public void accepts'
        artifact = 'openspec/specs/acceptance/spec.md'
        write(project / artifact, '### Requirement: Acceptance\nThe system MUST accept valid input.\n#### Scenario: Valid\n- **WHEN** valid input\n- **THEN** accepted\n')
        plan = {'schema_version': 'testguard.execution-plan/v1', 'task_id': 'REQ-LOOP-' + runner,
                'runner': runner, 'required_tests': [test_id], 'timeout_seconds': 180}
        plan_path = base / (runner + '-plan.json')
        write(plan_path, plan)
        digest = 'sha256:' + hashlib.sha256(json.dumps(plan, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        manifest_path = base / (runner + '-trace.json')
        write(manifest_path, {'schema_version': 'specguard.trace/v1', 'source': 'openspec',
                             'execution_plan_digest': digest, 'mappings': [{'artifact': artifact,
                             'requirement_id': 'Acceptance', 'code': [{'path': source.relative_to(project).as_posix(), 'marker': marker}], 'tests': [test_id]}]})
        policy_path = base / (runner + '-policy.json')
        write(policy_path, {'task_id': plan['task_id'], 'runner': runner, 'plan_digest': digest,
                            'required_tests': [test_id], 'max_age_seconds': 3600})
        # Cargo regression explicitly demonstrates failing code then repair and a new run.
        if runner == 'cargo':
            write(source, good.replace('-> bool { true }', '-> bool { false }'))
            failed = call('testguard', 'run', '--project', project, '--plan', plan_path,
                          '--output', base / (runner + '-failed'), '--executable', executable,
                          '--execute', '--private-key', private, code=2)
            assert failed.get('status', failed.get('payload', {}).get('status')) != 'PASS'
            write(source, good)
        output = base / (runner + '-passed')
        call('testguard', 'run', '--project', project, '--plan', plan_path,
             '--output', output, '--executable', executable, '--execute', '--private-key', private)
        receipt = output / 'receipt.json'
        call('testguard', 'verify', '--project', project, '--plan', plan_path,
             '--receipt', receipt, '--public-key', public)
        trace = call('specguard', 'trace', 'check', '--project', project, '--manifest', manifest_path,
                     '--evidence', receipt, '--public-key', public)
        assert trace['status'] == 'PASS'
        report_path = base / (runner + '-trace-report.json')
        write(report_path, trace)
        call('specguard', 'trace', 'verify', '--project', project, '--manifest', manifest_path,
             '--evidence', receipt, '--public-key', public, '--report', report_path)
        flow = call('flowguard', 'evidence', 'check', '--project', project, '--receipt', receipt,
                    '--policy', policy_path, '--trusted-key', public, code=3)
        assert flow['technical_pass'] is True and flow['decision'] == 'requires_approval'
        write(source, good + '\n// changed after execution\n')
        rejected = call('flowguard', 'evidence', 'check', '--project', project, '--receipt', receipt,
                        '--policy', policy_path, '--trusted-key', public, code=2)
        assert rejected['technical_pass'] is False
        write(source, good)
        restored = call('flowguard', 'evidence', 'check', '--project', project, '--receipt', receipt,
                        '--policy', policy_path, '--trusted-key', public, code=3)
        assert restored['technical_pass'] is True
        results.append({'runner': runner, 'execution': 'PASS', 'trace': 'PASS',
                        'flow': 'requires_approval', 'source_drift': 'rejected', 'restored': 'verified'})
    print(json.dumps({'fixtures': str(base), 'results': results}, indent=2))


if __name__ == '__main__':
    main()
