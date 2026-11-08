import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

interface Props {
  children: ReactNode;
  /** Ganti nilainya (mis. path halaman) untuk membersihkan galat saat pengguna berpindah halaman. */
  resetKey?: string;
}

interface State {
  error: Error | null;
  key: string | undefined;
}

/**
 * Menangkap galat render supaya satu halaman yang rusak tidak mengosongkan seluruh aplikasi.
 * Data tidak pernah hilang karena semuanya sudah tersimpan di database, bukan di layar.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null, key: this.props.resetKey };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
    // Berpindah halaman membersihkan galat sebelumnya.
    if (props.resetKey !== state.key) return { error: null, key: props.resetKey };
    return null;
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Galat tak terduga di antarmuka:", error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div
        role="alert"
        className="mx-auto max-w-lg space-y-4 rounded-xl border p-8 text-center"
        data-error-boundary
      >
        <h1 className="text-xl font-semibold">Terjadi kesalahan tak terduga</h1>
        <p className="text-sm text-muted-foreground">
          Halaman ini gagal ditampilkan. Data Anda aman karena sudah tersimpan di database.
        </p>
        <pre className="max-h-32 overflow-auto rounded-lg border bg-muted/30 p-3 text-left text-xs whitespace-pre-wrap">
          {error.message}
        </pre>
        <div className="flex justify-center gap-2">
          <Button variant="outline" onClick={() => this.setState({ error: null })}>
            Coba lagi
          </Button>
          <Button onClick={() => window.location.reload()}>Muat ulang aplikasi</Button>
        </div>
      </div>
    );
  }
}
