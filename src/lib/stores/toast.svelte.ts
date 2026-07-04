export type ToastType = "success" | "info" | "error";

export type ToastItem = {
  id: string;
  type: ToastType;
  message: string;
  detail?: string;
};

let toasts = $state<ToastItem[]>([]);
const MAX_VISIBLE = 3;

export const toastStore = {
  get toasts() {
    return toasts;
  },

  show(type: ToastType, message: string, detail?: string) {
    const id = crypto.randomUUID();
    toasts = [...toasts.slice(-(MAX_VISIBLE - 1)), { id, type, message, detail }];
    if (type !== "error") {
      setTimeout(() => {
        toasts = toasts.filter((t) => t.id !== id);
      }, 4000);
    }
  },

  dismiss(id: string) {
    toasts = toasts.filter((t) => t.id !== id);
  },
};
