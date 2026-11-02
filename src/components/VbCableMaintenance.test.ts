import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import VbCableMaintenance from './VbCableMaintenance.vue';

const mocks=vi.hoisted(()=>({getComponentStatus:vi.fn(),performComponentAction:vi.fn()}));
vi.mock('../lib/bridge',async importOriginal=>({...await importOriginal<typeof import('../lib/bridge')>(),...mocks}));
const status={component:'vb_cable',installation:'available',package:'download_available',installedVersion:null,serviceInstalled:true,loaded:true,bound:null,audioEndpointsReady:true,restartRequired:false,reason:'ready',blockers:['official_wizard_required'],allowedActions:['open_vendor_wizard']} as const;
const wrappers:ReturnType<typeof mount>[]=[];
async function panel(){const wrapper=mount(VbCableMaintenance);wrappers.push(wrapper);await flushPromises();return wrapper;}
function wizard(wrapper:ReturnType<typeof mount>){return wrapper.findAll('button').find(button=>button.text()==='打开官方安装/卸载向导')!;}

describe('VB-CABLE maintenance',()=>{
 beforeEach(()=>{vi.resetAllMocks();mocks.getComponentStatus.mockResolvedValue(status);});
 afterEach(()=>{for(const wrapper of wrappers.splice(0))wrapper.unmount();});
 it('reads component status without starting the official wizard',async()=>{
  const wrapper=await panel();expect(mocks.getComponentStatus).toHaveBeenCalledExactlyOnceWith('vb_cable');expect(mocks.performComponentAction).not.toHaveBeenCalled();expect(wrapper.text()).not.toContain('全按键支持');
 });
 it('does not equate closing the official wizard with installing a component',async()=>{
  const missing={...status,installation:'not_installed',audioEndpointsReady:false};mocks.getComponentStatus.mockResolvedValue(missing);
  mocks.performComponentAction.mockResolvedValue({component:'vb_cable',action:'open_vendor_wizard',outcome:'wizard_closed',reason:'wizard_closed',status:missing});
  const wrapper=await panel();await wizard(wrapper).trigger('click');await flushPromises();
  expect(mocks.performComponentAction).toHaveBeenCalledExactlyOnceWith('vb_cable','open_vendor_wizard');expect(wrapper.text()).toContain('向导已关闭');expect(wrapper.text()).toContain('需要检查');expect(wrapper.text()).not.toContain('安装成功');
 });
 it.each([
  ['cancelled','cancelled','已取消管理员授权'],
  ['timed_out','timeout','向导可能仍在运行'],
  ['failed','hash_mismatch','组件包校验未通过'],
 ] as const)('retains %s feedback after refreshing actual status',async(outcome,reason,message)=>{
  mocks.performComponentAction.mockResolvedValue({component:'vb_cable',action:'open_vendor_wizard',outcome,reason,status});const wrapper=await panel();await wizard(wrapper).trigger('click');await flushPromises();expect(wrapper.get('.component-message').text()).toContain(message);
 });
 it('keeps a read failure visible and does not invent an allowed action',async()=>{
  mocks.getComponentStatus.mockRejectedValue(new Error('read failed'));const wrapper=await panel();expect(wrapper.text()).toContain('暂时无法读取组件状态');expect(wizard(wrapper).attributes('disabled')).toBeDefined();expect(mocks.performComponentAction).not.toHaveBeenCalled();
 });
 it.each([['timed_out','helper_timed_out','向导可能仍在运行'],['failed','hash_mismatch','组件包校验未通过']] as const)('preserves %s when the following status read fails',async(outcome,reason,message)=>{
  mocks.getComponentStatus.mockResolvedValueOnce(status).mockRejectedValue(new Error('read failed'));
  mocks.performComponentAction.mockResolvedValue({component:'vb_cable',action:'open_vendor_wizard',outcome,reason,status});const wrapper=await panel();await wizard(wrapper).trigger('click');await flushPromises();
  expect(wrapper.get('.component-message').text()).toContain(message);expect(wrapper.get('.component-message').text()).toContain('暂时无法读取组件状态');expect(wizard(wrapper).attributes('disabled')).toBeDefined();
  expect(wrapper.get('.component-message').attributes('title')).toBe(wrapper.get('.component-message').text());
  expect(wrapper.get('.component-message').attributes('tabindex')).toBe('0');
 });
 it('does not repeat a busy request or refresh a component that was left',async()=>{
  let finish!:(value:unknown)=>void;mocks.performComponentAction.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const wrapper=await panel();await wizard(wrapper).trigger('click');expect(wizard(wrapper).attributes('disabled')).toBeDefined();await wizard(wrapper).trigger('click');expect(mocks.performComponentAction).toHaveBeenCalledTimes(1);
  wrapper.unmount();wrappers.splice(wrappers.indexOf(wrapper),1);finish({component:'vb_cable',action:'open_vendor_wizard',outcome:'wizard_closed',reason:'wizard_closed',status});await flushPromises();expect(mocks.getComponentStatus).toHaveBeenCalledTimes(1);
 });
});
