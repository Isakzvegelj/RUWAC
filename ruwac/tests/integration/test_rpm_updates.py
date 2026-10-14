#!/usr/bin/env python3
"""Generated package/update faults in private state; never changes host RPMs or timers."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import stat
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('rpm_updates', ROOT / 'scripts/rpm-update.py')
rpm = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(rpm)
SPEC = importlib.util.spec_from_file_location('updates', ROOT / 'scripts/update-user.py')
updates = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(updates)


class RPMUpdates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.owner = os.geteuid()
        self.config = self.root / 'etc/ruwac/rpm-updates.json'
        self.release = {'current':'0.2.0','latest':'0.2.1','tag':'v0.2.1','commit':'a'*40,'available':True}
        self.components = ['ruwac','ruwac-desktop']
        self.current = ['0.2.0']
        self.asset = lambda name: {'name':name,'state':'uploaded','size':13,
            'digest':'sha256:'+hashlib.sha256(b'generated RPM').hexdigest(),
            'browser_download_url':'https://github.com/w3fans/RUWAC/releases/download/v0.2.1/'+name}
        self.data = {'tag_name':'v0.2.1','draft':False,'prerelease':False,'assets':[
            self.asset(c+'-0.2.1-1.fc44.x86_64.rpm') for c in self.components]}
        self.helper = types.SimpleNamespace(version=updates.version, MAX_BUNDLE_BYTES=updates.MAX_BUNDLE_BYTES,
            latest=Mock(return_value=self.release), automatic_release_verified=Mock(return_value=True),
            api=Mock(return_value=self.data), download_bundle=Mock(side_effect=lambda a,p:p.write_bytes(b'generated RPM')),
            verify_release_provenance=Mock())
        self.engine = rpm.Updater(self.helper,self.root/'state',self.config,self.owner)
        self.engine.installed = Mock(side_effect=lambda:(self.current[0],self.components.copy()))
        self.commands = []
        def run(args,**kwargs):
            self.commands.append(args)
            if args[0]=='/usr/bin/dnf':self.current[0]='0.2.1'
            return subprocess.CompletedProcess(args,0,'','')
        self.process = patch.object(rpm.subprocess,'run',side_effect=run); self.process.start()
        self.metadata = patch.object(rpm,'query',side_effect=lambda a: ('ruwac-desktop'if Path(a[-1]).name.startswith('ruwac-desktop-')else'ruwac')+'\t0.2.1\t1.fc44\tx86_64\n')
        self.metadata.start()
        rpm.save_policy(True,self.config,self.owner)

    def tearDown(self):
        self.metadata.stop();self.process.stop();self.temp.cleanup()

    def test_both_verified_before_one_upgrade_without_downgrade(self):
        result = self.engine.install(True)
        self.assertEqual(result['installed'],'0.2.1')
        self.assertEqual([c.args[-1] for c in self.helper.verify_release_provenance.call_args_list],self.components)
        self.assertEqual(len(self.commands),1)
        self.assertEqual(self.commands[0][:4],['/usr/bin/dnf','-y','upgrade','--no-allow-downgrade'])
        self.assertEqual(len(self.commands[0]),6)
        self.assertFalse(any(p.name.startswith('verified-')for p in (self.root/'state').iterdir()))
        self.assertEqual((self.root/'state/last-transaction.log').stat().st_mode&0o777,0o600)

    def test_headless_installation_stays_headless(self):
        self.components=['ruwac']
        self.engine.install(True)
        self.assertEqual(len(self.commands[0]),5)
        self.assertEqual(self.helper.verify_release_provenance.call_args.args[-1],'ruwac')

    def test_disabled_auto_does_not_check_network(self):
        rpm.save_policy(False,self.config,self.owner)
        self.assertIn('skipped',self.engine.install(True))
        self.helper.latest.assert_not_called();self.assertEqual(self.commands,[])

    def test_quality_or_missing_component_holds_all_packages(self):
        self.helper.automatic_release_verified.return_value=False
        self.assertIn('skipped',self.engine.install(True))
        self.helper.automatic_release_verified.return_value=True
        self.data['assets']=self.data['assets'][:1]
        self.assertIn('skipped',self.engine.install(True))
        self.helper.download_bundle.assert_not_called();self.assertEqual(self.commands,[])

    def test_each_signature_failure_prevents_any_transaction(self):
        for failure in [0,1]:
            self.helper.verify_release_provenance.reset_mock()
            self.helper.verify_release_provenance.side_effect=[ValueError('generated bad signature')]if failure==0 else [None,ValueError('generated bad signature')]
            with self.assertRaises(ValueError):self.engine.install(True)
            self.assertEqual(self.commands,[])

    def test_invalid_or_ambiguous_assets(self):
        name=self.data['assets'][0]['name'];good=self.asset(name)
        for change in [{'size':True},{'size':0},{'digest':'sha256:'+'z'*64},
                       {'state':'starter'},{'browser_download_url':'https://example.invalid/generated'}]:
            with self.subTest(change=change),self.assertRaises(ValueError):
                self.engine.asset(self.release,dict(self.data,assets=[dict(good,**change)]),'ruwac')
        with self.assertRaises(ValueError):self.engine.asset(self.release,dict(self.data,assets=[good,good]),'ruwac')

    def test_opt_out_during_download_and_changed_installation_prevent_transaction(self):
        def optout(a,p):p.write_bytes(b'generated RPM');rpm.save_policy(False,self.config,self.owner)
        self.helper.download_bundle.side_effect=optout
        self.assertIn('skipped',self.engine.install(True));self.assertEqual(self.commands,[])
        rpm.save_policy(True,self.config,self.owner)
        def changed(a,p):p.write_bytes(b'generated RPM');self.current[0]='0.2.2'
        self.helper.download_bundle.side_effect=changed
        self.assertIn('skipped',self.engine.install(True));self.assertEqual(self.commands,[])

    def test_wrong_package_header_and_transaction_failure(self):
        with patch.object(rpm,'query',return_value='generated-other\t0.2.1\t1.fc44\tx86_64\n'),self.assertRaises(ValueError):
            self.engine.install(True)
        self.assertEqual(self.commands,[])
        with patch.object(rpm.subprocess,'run',side_effect=subprocess.CalledProcessError(1,['generated'])),self.assertRaisesRegex(ValueError,'transaction failed'):
            self.engine.install(True)
        self.assertEqual(self.current[0],'0.2.0')

    def test_configure_never_restarts_product_and_failed_enable_disables(self):
        self.engine.configure(True)
        self.assertEqual(self.commands,[['/usr/bin/systemctl','daemon-reload'],['/usr/bin/systemctl','enable','--now',rpm.TIMER]])
        with patch.object(rpm.subprocess,'run',side_effect=subprocess.CalledProcessError(1,['generated'])),self.assertRaises(ValueError):
            self.engine.configure(True)
        self.assertFalse(rpm.policy(self.config,self.owner))
        self.assertEqual(self.config.stat().st_mode&0o777,0o644)

    def test_unsafe_policy_symlink_hardlink_permissions_and_malformed_json(self):
        self.config.unlink();target=self.root/'sentinel';target.write_text('preserve');self.config.symlink_to(target)
        with self.assertRaises(ValueError):rpm.save_policy(True,self.config,self.owner)
        self.assertEqual(target.read_text(),'preserve');self.config.unlink()
        os.link(target,self.config)
        with self.assertRaises(ValueError):rpm.policy(self.config,self.owner)
        self.config.unlink();self.config.write_text('{}');self.config.chmod(0o666)
        with self.assertRaises(ValueError):rpm.policy(self.config,self.owner)
        self.config.chmod(0o644)
        with self.assertRaises(ValueError):rpm.policy(self.config,self.owner)

    def test_busy_transaction_does_not_queue_or_check_network(self):
        rpm.directory(self.engine.state,self.owner,private=True)
        with rpm.lock(self.engine.state/'.transaction.lock',self.owner):
            with self.assertRaisesRegex(ValueError,'owns the transaction'):self.engine.install(True)
        self.helper.latest.assert_not_called()

    def test_unsafe_log_preserves_link_target(self):
        target=self.root/'preserve-log';target.write_text('generated sentinel');target.chmod(0o600)
        log=self.root/'log';log.symlink_to(target)
        with self.assertRaises(OSError):
            with rpm.transaction_log(log,self.owner):pass
        log.unlink();os.link(target,log)
        with self.assertRaises(ValueError):
            with rpm.transaction_log(log,self.owner):pass
        self.assertEqual(target.read_text(),'generated sentinel')


class RPMBridge(unittest.TestCase):
    def setUp(self):
        def metadata(path):
            kind=stat.S_IFREG if str(path)=='/usr/share/ruwac/rpm-update.py'else stat.S_IFDIR
            return types.SimpleNamespace(st_uid=0,st_mode=kind|0o755)
        self.exists=patch.object(Path,'exists',return_value=True);self.exists.start()
        self.resolve=patch.object(Path,'resolve',return_value=Path('/usr/share/ruwac/rpm-update.py'));self.resolve.start()
        self.metadata=patch.object(Path,'lstat',autospec=True,side_effect=metadata);self.lstat=self.metadata.start()
        self.uid=patch.object(updates.os,'geteuid',return_value=42);self.uid.start()
        self.process=patch.object(updates.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'{"distribution":"rpm","automatic_install":false}',''));self.run=self.process.start()

    def tearDown(self):
        self.process.stop();self.uid.stop();self.metadata.stop();self.resolve.stop();self.exists.stop()

    def test_status_is_unprivileged_and_mutations_use_only_fixed_helper_flags(self):
        updates.rpm_operation('status')
        self.assertEqual(self.run.call_args.args[0],['/usr/bin/ruwac-rpm-update','--status'])
        for mode,flag in [('on','--enable-testing'),('off','--disable'),('install','--install')]:
            updates.rpm_operation(mode)
            self.assertEqual(self.run.call_args.args[0],['/usr/bin/pkexec','/usr/bin/ruwac-rpm-update',flag])

    def test_unsafe_helper_or_directory_never_requests_authorization(self):
        for target in ['/usr/share/ruwac/rpm-update.py','/usr/share/ruwac','/usr/bin']:
            def metadata(path):
                kind=stat.S_IFREG if str(path)=='/usr/share/ruwac/rpm-update.py'else stat.S_IFDIR
                return types.SimpleNamespace(st_uid=0,st_mode=kind|(0o777 if str(path)==target else 0o755))
            self.lstat.side_effect=metadata
            with self.assertRaises(ValueError):updates.rpm_operation('on')
        self.run.assert_not_called()

    def test_refusal_invalid_response_and_unsupported_operation_hold_install(self):
        for code,body in [(1,'{"error":"generated"}'),(0,'{}'),(0,'x'*32769)]:
            self.run.return_value=subprocess.CompletedProcess([],code,body,'generated private diagnostic')
            with self.assertRaises(ValueError):updates.rpm_operation('install')
        self.run.reset_mock()
        with self.assertRaises(ValueError):updates.rpm_operation('generated-extra-operation')
        self.run.assert_not_called()


if __name__=='__main__':unittest.main()
