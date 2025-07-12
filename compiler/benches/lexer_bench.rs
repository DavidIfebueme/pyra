use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pyra_compiler::lexer::PyraLexer;

fn lexer_benchmark(c: &mut Criterion) {
    // 🔥 PHASE 1 COMPREHENSIVE STRESS TEST
    // Tests ALL fixes: BigUint, Bytes, Indentation, Error Handling
    let large_contract = r#"
contract ComplexERC20Token:
    # Storage with massive numbers and addresses
    balances: mapping[address => uint256]
    allowances: mapping[address => mapping[address => uint256]]
    
    # Constants with huge BigUint values (Fix #1: BigUint support)
    TOTAL_SUPPLY: constant(uint256) = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    MAX_MINT: constant(uint256) = 115792089237316195423570985008687907853269984665640564039457584007913129639935
    
    # Events with complex types
    event Transfer(indexed from: address, indexed to: address, value: uint256)
    event Approval(indexed owner: address, indexed spender: address, value: uint256)

    def __init__(initial_supply: uint256):
        """Initialize contract with massive initial supply"""
        self.balances[msg.sender] = initial_supply * 1000000000000000000  # 18 decimals
        
        # Complex hex calculations with addresses (Fix #1: BigUint edge cases)
        if initial_supply > 0xde0b6b3a7640000:  # 1 ETH in wei
            self.allowances[msg.sender][0x742d35Cc6032C0532] = 0x8ac7230489e80000  # 10 ETH
    
    def transfer(to: address, amount: uint256) -> bool:
        """Transfer tokens with complex validation"""
        # Multi-level indentation stress test (Fix #3: Indentation validation)
        if amount > 0:
            if self.balances[msg.sender] >= amount:
                if to != empty(address):
                    if amount <= 0xffffffffffffffffffffffffffffffff:  # Huge hex number
                        # Bytes literals for transaction signatures (Fix #2: Byte literals)
                        signature_hash: bytes32 = keccak256(
                            concat(
                                b'1901',  # EIP-191 prefix
                                b'0001',  # Version
                                b'deadbeef',  # Domain separator part
                                b'abcdef1234567890'  # Transfer hash
                            )
                        )
                        
                        # Complex nested operations
                        self.balances[msg.sender] -= amount
                        self.balances[to] += amount
                        
                        # Log with massive values
                        log Transfer(msg.sender, to, amount)
                        return True
                    else:
                        # Error case for testing (Fix #4: Error granularity would catch issues here)
                        raise "Amount exceeds maximum allowed value"
                else:
                    raise "Cannot transfer to zero address"
            else:
                raise "Insufficient balance for transfer"
        else:
            raise "Transfer amount must be positive"
        
        return False
    
    def approve(spender: address, amount: uint256) -> bool:
        """Approve spending with cryptographic validation"""
        # Complex conditional logic with mixed operators
        if spender != empty(address) and amount >= 0 and amount <= TOTAL_SUPPLY:
            # Bytes manipulation for signature verification
            approval_data: bytes = concat(
                b'approve',
                convert(spender, bytes20),
                convert(amount, bytes32),
                b'cafebabe'  # Nonce
            )
            
            # Massive hex numbers in calculations
            fee: uint256 = amount * 0x16345785d8a0000 / 0xde0b6b3a7640000  # Complex fee calc
            
            if self.balances[msg.sender] >= fee:
                self.balances[msg.sender] -= fee
                self.allowances[msg.sender][spender] = amount
                
                log Approval(msg.sender, spender, amount)
                return True
        
        return False
    
    def transferFrom(from_addr: address, to: address, amount: uint256) -> bool:
        """Transfer from with maximum complexity"""
        # Deep nesting for indentation stress testing
        if from_addr != empty(address):
            if to != empty(address):
                if amount > 0:
                    if self.balances[from_addr] >= amount:
                        if self.allowances[from_addr][msg.sender] >= amount:
                            # Bytes operations for event logging
                            transfer_id: bytes32 = keccak256(concat(
                                convert(from_addr, bytes20),
                                convert(to, bytes20),
                                convert(amount, bytes32),
                                convert(block.timestamp, bytes32),
                                b'transferFrom'
                            ))
                            
                            # Execute transfer
                            self.balances[from_addr] -= amount
                            self.balances[to] += amount
                            self.allowances[from_addr][msg.sender] -= amount
                            
                            log Transfer(from_addr, to, amount)
                            return True
        
        return False
    
    def mint(to: address, amount: uint256) -> bool:
        """Mint tokens with overflow protection"""
        # Maximum BigUint stress test
        max_supply: uint256 = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        current_total: uint256 = self.get_total_supply()
        
        if current_total + amount <= max_supply:
            # Complex bytes hash for mint authorization
            mint_hash: bytes32 = keccak256(concat(
                b'mint',
                convert(to, bytes20),
                convert(amount, bytes32),
                convert(msg.sender, bytes20),
                b'deadbeefcafebabe1234567890abcdef'
            ))
            
            self.balances[to] += amount
            log Transfer(empty(address), to, amount)
            return True
        
        return False
    
    def get_total_supply() -> uint256:
        """Calculate total supply with massive iterations"""
        total: uint256 = 0
        
        # This would be inefficient in real code but tests our lexer
        for i in range(1000):
            if i % 2 == 0:
                total += 0x16345785d8a0000  # Add large hex
            else:
                total += 1000000000000000000  # Add 1 ETH in wei
        
        return total
    
    def complex_calculation() -> bytes:
        """Complex function mixing all our lexer features"""
        # Massive hex numbers (BigUint test)
        value1: uint256 = 0x8ac7230489e80000de0b6b3a7640000cafebabe
        value2: uint256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935
        
        # Complex bytes operations
        result: bytes = concat(
            b'complex',
            convert(value1 + value2, bytes32),
            b'calculation',
            b'0123456789abcdef' * 4,  # Long bytes literal
            convert(block.timestamp, bytes32)
        )
        
        return result

# Multiple contracts for extra stress testing
contract TokenFactory:
    def deploy_token(name: string, symbol: string, supply: uint256) -> address:
        # More complex operations...
        return create_forwarder_to(self)
"#.repeat(50);  // 50 copies for serious stress testing

    c.bench_function("lexer_comprehensive_stress_test", |b| {
        b.iter(|| {
            let mut lexer = PyraLexer::new(black_box(&large_contract));
            let _tokens: Vec<_> = lexer.collect();
        })
    });
    
    // Additional benchmark for error handling performance
    let error_test_contract = r#"
        def error_prone_function():
            # Mix of valid and edge-case tokens that stress error detection
            value = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
            bytes_data = b'deadbeefcafebabedeadbeefcafebabedeadbeefcafebabe'
            
            if value > 0:
                    nested_operation()  # Mixed indentation styles
                    another_operation()  # Tab vs spaces
                final_operation()
    "#.repeat(200);
    
    c.bench_function("lexer_error_handling_stress", |b| {
        b.iter(|| {
            let mut lexer = PyraLexer::new(black_box(&error_test_contract));
            let _tokens: Vec<_> = lexer.collect();
        })
    });
}

criterion_group!(benches, lexer_benchmark);
criterion_main!(benches);