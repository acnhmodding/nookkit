// Args: out_tsv
// @category ACNH

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.nio.file.*;
import java.util.*;

public class ExportPageClasses extends GhidraScript {

    @Override
    public void run() throws Exception {
        List<String> out = new ArrayList<>();
        out.add("# class\tlayout\tvtable");
        int skipped = 0;

        for (Symbol s : currentProgram.getSymbolTable().getSymbols("vtable")) {
            String cls = s.getParentNamespace().getName(true);
            if (!cls.endsWith("Page")) {
                continue;
            }
            Address vt = s.getAddress();
            String name = literal(vt.add(0x20));
            String layout = literal(vt.add(0x170));

            if (name == null || layout == null) {
                println("skip " + cls + " @" + vt + ": class " + name + ", layout " + layout);
                skipped++;

                continue;
            }

            out.add(name + "\t" + layout + "\t" + vt);
        }

        Files.write(Paths.get(getScriptArgs()[0]), out);
        println("exported " + (out.size() - 1) + ", skipped " + skipped);
    }

    private String literal(Address slot) throws Exception {
        Function f = getFunctionAt(toAddr(getLong(slot)));
        if (f == null) {
            return null;
        }
        Set<String> found = new TreeSet<>();

        for (Instruction ins : currentProgram.getListing().getInstructions(f.getBody(), true)) {
            // adrp only points at the 4 KiB page, often mid-string, the following add carries the literal
            if (ins.getMnemonicString().equals("adrp")) {
                continue;
            }

            for (Reference r : ins.getReferencesFrom()) {
                String s = identifier(r.getToAddress());
                if (s != null) {
                    found.add(s);
                }
            }
        }

        return found.size() == 1 ? found.iterator().next() : null;
    }

    // many literals are not defined as string data, so read an identifier-like C string directly
    private String identifier(Address at) {
        StringBuilder sb = new StringBuilder();

        try {
            for (int i = 0; i < 128; i++) {
                byte b = getByte(at.add(i));
                if (b == 0) {
                    return sb.length() >= 2 ? sb.toString() : null;
                }
                if (!Character.isLetterOrDigit(b) && b != '_') {
                    return null;
                }

                sb.append((char) b);
            }
        } catch (Exception e) {
            return null;
        }

        return null;
    }
}
