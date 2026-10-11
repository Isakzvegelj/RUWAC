#!/usr/bin/env python3
"""Synthetic API/event tests for the privileged base-only label workflow."""
import importlib.util,json,os,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('label_pr',Path(__file__).with_name('label-pr.py'));label=importlib.util.module_from_spec(spec);spec.loader.exec_module(label)
class Labels(unittest.TestCase):
 def test_area_classification_and_plain_data(self):
  self.assertEqual(label.classify(['ruwac/desktop/src/rdp_wire.rs','ruwac/crates/ruwac-clipboard/src/lib.rs','.github/workflows/quality.yml','CONTRIBUTING.md']),['area/ci','area/clipboard','area/docs','area/rdp','area/ui'])
  self.assertEqual(label.classify(['$(touch ignored).txt']),[])
 def test_traversal_is_refused(self):
  for p in ['/tmp/private','../private','dir/../private']:
   with self.assertRaises(ValueError):label.classify([p])
 def invoke(self,event,head='trusted-base',fail=False):
  with tempfile.TemporaryDirectory(prefix='label-event-')as root:
   event_path=Path(root)/'event.json';event_path.write_text(json.dumps(event));calls=[]
   def api(method,path,body=None):
    calls.append((method,path,body))
    if '/files?' in path:return [{'filename':'ruwac/desktop/src/files.rs'}]
    if path.startswith('/labels?'):return [{'name':'human-label'},{'name':'area/ui'}]
    if method=='POST':return {}
    raise AssertionError('Unexpected API operation')
   with patch.dict(os.environ,{'GITHUB_REPOSITORY':'w3fans/RUWAC','GITHUB_EVENT_PATH':str(event_path)}),patch.object(sys,'argv',['label-pr.py']),patch.object(subprocess,'check_output',return_value=head+'\n'),patch.object(label,'api',side_effect=api):
    if fail:
     with self.assertRaises(ValueError):label.main()
    else:label.main()
   return calls
 def event(self):return {'number':7,'repository':{'full_name':'w3fans/RUWAC'},'pull_request':{'base':{'sha':'trusted-base','repo':{'full_name':'w3fans/RUWAC'}},'head':{'repo':{'full_name':'untrusted/fork'}},'changed_files':1}}
 def test_fork_uses_base_and_adds_only_managed_labels(self):
  calls=self.invoke(self.event());self.assertEqual(calls[-1],('POST','/issues/7/labels',{'labels':['area/files','area/ui']}));self.assertFalse(any(c[0]=='DELETE'for c in calls));self.assertEqual([c[2]['name']for c in calls if c[0]=='POST'and c[1]=='/labels'],['area/files'])
 def test_untrusted_checkout_has_no_api_side_effects(self):self.assertEqual(self.invoke(self.event(),'untrusted-head',True),[])
 def test_wrong_repo_and_boolean_number_refused(self):
  e=self.event();e['repository']['full_name']='other/project';self.assertEqual(self.invoke(e,fail=True),[])
  e=self.event();e['number']=True;self.assertEqual(self.invoke(e,fail=True),[])
 def test_incomplete_files_do_not_apply_labels(self):
  e=self.event();e['pull_request']['changed_files']=2;calls=self.invoke(e,fail=True);self.assertTrue(all(c[0]=='GET'for c in calls))
if __name__=='__main__':unittest.main()
