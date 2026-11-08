import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import {
  ACCOUNT_TYPE_LABEL,
  api,
  defaultNormalBalance,
  errorMessage,
  type Account,
  type AccountInput,
  type AccountType,
} from "@/lib/api";

export type DialogMode =
  { kind: "create"; parent: Account | null } | { kind: "edit"; account: Account };

const TYPES = Object.keys(ACCOUNT_TYPE_LABEL) as AccountType[];

interface Props {
  mode: DialogMode;
  accounts: Account[];
  onClose: () => void;
}

/** Form tambah/ubah akun. Dipasang ulang (key) setiap dibuka sehingga state selalu segar. */
export function AccountDialog({ mode, accounts, onClose }: Props) {
  const queryClient = useQueryClient();
  const editing = mode.kind === "edit" ? mode.account : null;
  const initialParent = mode.kind === "create" ? mode.parent : null;

  const [code, setCode] = useState(editing?.code ?? "");
  const [name, setName] = useState(editing?.name ?? "");
  const [type, setType] = useState<AccountType>(editing?.type ?? initialParent?.type ?? "ASSET");
  const [parentId, setParentId] = useState<number | null>(
    editing ? editing.parentId : (initialParent?.id ?? null),
  );
  const [isHeader, setIsHeader] = useState(editing?.isHeader ?? false);
  const [isCash, setIsCash] = useState(editing?.isCash ?? false);
  const [contra, setContra] = useState(
    editing ? editing.normalBalance !== defaultNormalBalance(editing.type) : false,
  );

  const locked = editing?.isSystem ?? false; // akun sistem: hanya kode & nama
  const typeLocked = locked || parentId !== null || (editing?.inUse ?? false);
  const headerLocked = locked || (editing?.inUse ?? false);

  const parentOptions = accounts.filter(
    (a) => a.isHeader && a.isActive && a.id !== editing?.id && a.type === type,
  );

  const save = useMutation({
    mutationFn: () => {
      const input: AccountInput = {
        code,
        name,
        type,
        normalBalance: contra
          ? defaultNormalBalance(type) === "DEBIT"
            ? "CREDIT"
            : "DEBIT"
          : null,
        parentId,
        isHeader,
        isCash: isCash && type === "ASSET" && !isHeader,
      };
      return editing ? api.updateAccount(editing.id, input) : api.createAccount(input);
    },
    onSuccess: async (acc) => {
      await queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(editing ? `Akun ${acc.code} diperbarui` : `Akun ${acc.code} ditambahkan`);
      onClose();
    },
  });

  function changeParent(value: string) {
    const id = value === "" ? null : Number(value);
    setParentId(id);
    const parent = accounts.find((a) => a.id === id);
    if (parent) setType(parent.type);
  }

  function changeType(next: AccountType) {
    setType(next);
    setParentId(null); // induk harus sejenis; pilih ulang
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          <DialogHeader>
            <DialogTitle>{editing ? "Ubah akun" : "Tambah akun"}</DialogTitle>
            <DialogDescription>
              {locked
                ? "Akun sistem dipakai otomatis oleh aplikasi; hanya kode dan nama yang bisa diubah."
                : "Akun header dipakai untuk mengelompokkan dan tidak bisa dijurnal."}
            </DialogDescription>
          </DialogHeader>

          <div className="grid grid-cols-[8rem_1fr] gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="acc-code">Kode</Label>
              <Input
                id="acc-code"
                autoFocus
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="1-1110"
                maxLength={20}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="acc-name">Nama akun</Label>
              <Input
                id="acc-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={100}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="acc-type">Jenis</Label>
              <NativeSelect
                id="acc-type"
                value={type}
                disabled={typeLocked}
                onChange={(e) => changeType(e.target.value as AccountType)}
              >
                {TYPES.map((t) => (
                  <option key={t} value={t}>
                    {ACCOUNT_TYPE_LABEL[t]}
                  </option>
                ))}
              </NativeSelect>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="acc-parent">Akun induk</Label>
              <NativeSelect
                id="acc-parent"
                value={parentId ?? ""}
                disabled={locked}
                onChange={(e) => changeParent(e.target.value)}
              >
                <option value="">(tanpa induk)</option>
                {parentOptions.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.code} {a.name}
                  </option>
                ))}
              </NativeSelect>
            </div>
          </div>

          <div className="space-y-2 text-sm">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={isHeader}
                disabled={headerLocked}
                onChange={(e) => setIsHeader(e.target.checked)}
              />
              Akun header (pengelompokan, tidak bisa dijurnal)
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={contra}
                disabled={locked || isHeader}
                onChange={(e) => setContra(e.target.checked)}
              />
              Akun kontra (saldo normal{" "}
              {defaultNormalBalance(type) === "DEBIT" ? "kredit" : "debit"}, mis. Akumulasi
              Penyusutan)
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={isCash && type === "ASSET" && !isHeader}
                disabled={type !== "ASSET" || isHeader}
                onChange={(e) => setIsCash(e.target.checked)}
              />
              Akun kas/bank (dihitung pada kartu Kas &amp; Bank di Dashboard)
            </label>
          </div>

          {save.error && <p className="text-sm text-destructive">{errorMessage(save.error)}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Batal
            </Button>
            <Button type="submit" disabled={save.isPending || !code.trim() || !name.trim()}>
              {save.isPending ? "Menyimpan..." : "Simpan"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
