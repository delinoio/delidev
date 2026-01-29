import { NextRequest, NextResponse } from "next/server";

const PRODUCT_IDS = [
    process.env.POLAR_LICENSE_PRODUCT_ID ?? '',
];

export async function GET(request: NextRequest) {
    const checkoutUrl = new URL('/checkout', request.url);
    checkoutUrl.searchParams.set('products', PRODUCT_IDS.join(','));
    return NextResponse.redirect(checkoutUrl);
}