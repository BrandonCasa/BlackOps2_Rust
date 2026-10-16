"""Offline source-instruction port: no runtime interpreter or fallback shader.

Emit static WESL from the exact recovered DXBC instruction sequence, replacing
only source binding/entrypoint boundaries. Immediate operands come from SHDR
bytes, never the disassembler's rounded decimal display.
"""
from pathlib import Path
import re, struct, math
def immediates(path):
    data=Path(path).read_bytes()
    count=struct.unpack_from('<I',data,28)[0]
    for offset in struct.unpack_from('<'+'I'*count,data,32):
        if data[offset:offset+4] not in (b'SHDR',b'SHEX'): continue
        size=struct.unpack_from('<I',data,offset+4)[0]
        words=struct.unpack_from('<'+'I'*(size//4),data,offset+8)
        result=[]; i=2
        while i<len(words):
            length=(words[i]>>24)&127
            if words[i] & 0x7ff == 53: # DXBC custom data, including icb.
                i += words[i + 1]
                continue
            assert length>0
            instruction=words[i:i+length]
            k=1
            while k<len(instruction):
                token=instruction[k]
                if token in (0x4001,0x4002):
                    n=1 if token==0x4001 else 4
                    result.append(instruction[k+1:k+1+n]); k+=n
                k+=1
            i+=length
        return result
    raise AssertionError(path)

def literal(words):
    values=[struct.unpack('<f',struct.pack('<I',word))[0] for word in words]

    if not all(math.isfinite(v) for v in values):
        return 'bitcast<vec4<f32>>(vec4<u32>(' + ', '.join(hex(v)+'u' for v in words) + '))'
    return 'vec4<f32>('+', '.join(repr(v) for v in values)+')'

def splitargs(text):
    return [p.strip() for p in re.split(r',\s*(?![^()]*\))',text)]

def signature(text,which):
    text=text.split('// '+which+' signature:')[1].split('// '+('Output' if which=='Input' else 'Patch')+' signature:')[0]
    result={}
    for name,index,mask,register,system,kind in re.findall(r'^//\s+(\w+)\s+(\d+)\s+([xyzw]+)\s+(\d+)\s+(\w+)\s+(\w+)',text,re.M):
        result[int(register)]=(name,int(index),mask,system)
    return result

def resources(text):
    found={}
    for name,kind,dimension,letter,index in re.findall(r'^//\s+(\w+)\s+(sampler|texture)\s+\w+\s+(\w+)\s+([st])(\d+)\s+1',text,re.M):
        found[letter+index]=(name,dimension)
    return found

class Port:
    def __init__(self, evidence,stage):
        self.stage=stage; self.text=Path(evidence['asm']).read_text().rstrip('\0')
        self.body=self.text.split(stage+'_4_0\n')[1]
        self.immediates=iter(immediates(evidence['path']))
        self.lines=[]; self.temp=0
        self.inputs=signature(self.text,'Input'); self.outputs=signature(self.text,'Output')
        self.resources=resources(self.text)
        self.input_builtin=next((i for i,v in self.inputs.items() if v[3]=='POS'),None)
        self.output_builtin=next((i for i,v in self.outputs.items() if v[3]=='POS'),None)
    def operand(self,arg,integer=False):
        neg=arg.startswith('-'); arg=arg.removeprefix('-')
        absolute=arg.startswith('|'); arg=arg.strip('|')
        if arg.startswith('l('):
            words=next(self.immediates)
            value=literal(words)
        else:
            m=re.fullmatch(r'(cb(\d)\[(\d+)\]|[rvo]\d+)(?:\.([xyzw]+))?',arg)
            assert m,(self.stage,arg)
            base,cb,index,swizzle=m.groups()
            if cb:
                index=int(index)
                if cb=='0': value=self.code_constants[index]
                elif cb=='3':
                    assert 0 <= index < 4
                    value=f'transform.world_rows[{index}]'
                else: raise AssertionError((cb,index))
            elif base[0]=='v' and self.stage=='vs':
                name,idx,_,_=self.inputs[int(base[1:])]
                if name=='POSITION': value='input.position'
                elif name=='COLOR': value='input.color'
                elif name=='NORMAL': value='input.normal'
                elif name=='TEXCOORD' and idx==0: value='vec4<f32>(input.uv, 0.0, 0.0)'
                elif name=='TEXCOORD' and idx==2: value='input.tangent'
                else: raise AssertionError((name,idx))
            elif base[0]=='v': value='input.position' if int(base[1:])==self.input_builtin else 'input.'+base
            else: value=base
            if swizzle: value='('+value+').'+swizzle.ljust(4,swizzle[-1])
        if integer: value='bitcast<vec4<i32>>('+value+')'
        if absolute: value='abs('+value+')'
        if neg: value='-('+value+')'
        return value
    def write(self,dest,expression):
        if dest=='null': return
        base,_,mask=dest.partition('.')
        mask=mask or 'xyzw'
        temp=f'instruction_{self.temp}'; self.temp+=1
        self.lines.append(f'    let {temp} = {expression};')
        if mask=='xyzw': self.lines.append(f'    {base} = {temp};')
        else:
            components=[f'{temp}.{c}' if c in mask else f'{base}.{c}' for c in 'xyzw']
            self.lines.append(f'    {base} = vec4<f32>('+', '.join(components)+');')
    def translate(self):
        registers=sorted(set(re.findall(r'\b([ro]\d+)\b',self.body)),key=lambda r:(r[0],int(r[1:])))
        self.lines += [f'    var {r}: vec4<f32>;' for r in registers]
        for line in self.body.splitlines():
            line=line.strip()
            if not line or line.startswith(('dcl_','//')) or line=='ret':continue
            if line == 'else':
                self.lines.append('    } else {'); continue
            if line == 'endif':
                self.lines.append('    }'); continue
            opcode,args=line.split(' ',1)
            args=splitargs(args)
            self.lines.append('    // '+line)
            saturated=opcode.endswith('_sat'); opcode=opcode.removesuffix('_sat')
            dest=args[0]
            if opcode in ('if_nz', 'if_z'):
                value=self.operand(dest)
                compare='!=' if opcode=='if_nz' else '=='
                self.lines.append(f'    if bitcast<vec4<u32>>({value}).x {compare} 0u {{'); continue
            if opcode=='sincos':
                value=self.operand(args[2]); t=f'angle{self.temp}'
                self.lines.append(f'    let {t} = {value};')
                self.write(args[0],f'sin({t})'); self.write(args[1],f'cos({t})'); continue
            if opcode in ('sample','sample_l'):
                coord=self.operand(args[1]); texture=args[2].split('.')[0]; samp=args[3]
                dims=self.resources[texture][1]; coords='xyz' if dims in ('cube','3d') else 'xy'
                texvar=self.texture_vars[texture]; sampler=self.sampler_vars[samp]
                rhs=f'textureSample({texvar}, {sampler}, ({coord}).{coords})'
                if opcode=='sample_l':
                    lod=self.operand(args[4]); rhs=f'textureSampleLevel({texvar}, {sampler}, ({coord}).{coords}, ({lod}).x)'
            elif opcode=='iadd':
                a,b=[self.operand(a,True) for a in args[1:]]
                rhs=f'bitcast<vec4<f32>>({a} + {b})'
            else:
                operands=[self.operand(a) for a in args[1:]]
                a=operands[0]
                b=operands[1] if len(operands)>1 else None
                c=operands[2] if len(operands)>2 else None
                if opcode=='mov':rhs=a
                elif opcode=='add':rhs=f'({a}) + ({b})'
                elif opcode=='mul':rhs=f'({a}) * ({b})'
                elif opcode=='div':rhs=f'({a}) / ({b})'
                elif opcode=='mad':rhs=f'fma({a}, {b}, {c})'
                elif opcode in ('dp2','dp3','dp4'):
                    swizzle='xyzw'[:int(opcode[-1])];rhs=f'vec4<f32>(dot(({a}).{swizzle}, ({b}).{swizzle}))'
                elif opcode in ('sqrt','max','min'):
                    rhs=f'{opcode}({a}'+(f', {b}' if b else '')+')'
                elif opcode in ('rsq','exp','log','frc'):
                    function={'rsq':'inverseSqrt','exp':'exp2','log':'log2','frc':'fract'}[opcode];rhs=f'{function}({a})'
                elif opcode in ('lt','ge','eq','ne'):
                    op={'lt':'<','ge':'>=','eq':'==','ne':'!='}[opcode]
                    rhs=f'bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ({a}) {op} ({b})))'
                elif opcode=='movc':rhs=f'select({c}, {b}, bitcast<vec4<u32>>({a}) != vec4<u32>(0u))'
                elif opcode in ('and','or'):
                    op='&' if opcode=='and' else '|'
                    rhs=f'bitcast<vec4<f32>>(bitcast<vec4<u32>>({a}) {op} bitcast<vec4<u32>>({b}))'
                elif opcode=='round_ni':rhs=f'floor({a})'
                elif opcode=='ftoi':rhs=f'bitcast<vec4<f32>>(vec4<i32>({a}))'
                elif opcode=='itof':rhs=f'vec4<f32>(bitcast<vec4<i32>>({a}))'
                else:raise AssertionError((opcode,line))
            if saturated:rhs=f'clamp({rhs}, vec4<f32>(0.0), vec4<f32>(1.0))'
            self.write(dest,rhs)
        assert next(self.immediates,None) is None, ('unconsumed source immediates',self.stage)
        return '\n'.join(self.lines)
