export function canReadInvoice(userId: string, tenant_id: string): boolean {
  return Boolean(userId && tenant_id);
}
