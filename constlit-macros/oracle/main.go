// Command constlit-oracle writes the TSV fixtures that constlit-macros' tests
// compare against: Kubernetes' own answers for quantities, names and selectors.
//
// Regenerate from this directory with
//
//	go run . quantity > ../src/testdata/k8s_quantity.tsv
//	go run . names > ../src/testdata/k8s_names.tsv
//	go run . selector > ../src/testdata/k8s_selector.tsv
//	go run . digest > ../src/testdata/oci_digest.tsv
//	go run . image > ../src/testdata/oci_image.tsv
//	go run . platform > ../src/testdata/oci_platform.tsv
//
// Inputs never contain tabs or newlines, so each line splits on tabs.
package main

import (
	// go-digest only accepts algorithms whose hash is linked in.
	_ "crypto/sha256"
	_ "crypto/sha512"
	"fmt"
	"math/rand"
	"os"
	"os/exec"
	"runtime"
	"runtime/debug"
	"strings"

	"github.com/containerd/platforms"
	"github.com/distribution/reference"
	"github.com/opencontainers/go-digest"
	"k8s.io/apimachinery/pkg/api/resource"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/util/validation"
)

// depVersion is the version of a dependency this binary was built with.
func depVersion(path string) string {
	info, ok := debug.ReadBuildInfo()
	if ok {
		for _, dep := range info.Deps {
			if dep.Path == path {
				return dep.Version
			}
		}
	}
	return "unknown"
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: constlit-oracle quantity|names|selector|digest|image|platform")
		os.Exit(2)
	}
	// The first line names what produced the verdicts.
	switch os.Args[1] {
	case "quantity", "names", "selector":
		fmt.Printf("# k8s.io/apimachinery %s\n", depVersion("k8s.io/apimachinery"))
	case "digest":
		fmt.Printf("# github.com/opencontainers/go-digest %s\n", depVersion("github.com/opencontainers/go-digest"))
	case "image":
		fmt.Printf("# github.com/distribution/reference %s\n", depVersion("github.com/distribution/reference"))
	case "platform":
		fmt.Printf("# github.com/containerd/platforms %s, %s\n", depVersion("github.com/containerd/platforms"), runtime.Version())
	}
	switch os.Args[1] {
	case "quantity":
		quantities()
	case "names":
		names()
	case "selector":
		selectors()
	case "digest":
		digests()
	case "image":
		images()
	case "platform":
		platformsFixture()
	default:
		fmt.Fprintln(os.Stderr, "unknown subcommand", os.Args[1])
		os.Exit(2)
	}
}

// Quantity inputs: the plan's tables plus generated ones.
var fixedQuantities = []string{
	"", " 1Gi", "1 Gi", "Gi", ".", "+", "x", "1K", "1g", "1t", "1p", "1mi", "1KI", "1gi",
	"1KiB", "1MB", "1GiB", "1e", "1E+", "1e-", "1e3.5", "1e3x", "1.2.3", "1µ", "0.1m", "100n",
	"1.0005", "-0.1m", "8Ei", "-8Ei", "1e400", "9223372036854775808", "1e-400",
	"-1", "2Gi", "1Ki", "-1Gi", "500m", "2000m", "1500m", "5u", "5n", "1.5", "0.1Ki",
	"1", "1.5Gi", "128Mi", "1e3", "1E3", "1E+3", "1e-3", "1E", "1Ei", "+1k", "-1.5", ".5", "5.",
	"1000000n", "0000000000000000000000000001", "1.000000000000000000000000000000",
	"1000000000000000000000000n", "9223372036854775807", "0e-400", "7Ei", "-1.5Gi",
	"250m", "512Mi", "1G", "0.5Ki", "0", "2", "1024", "0.5", "1m", "-0", "+0.5m",
	"9223372036854775807000m", "9223372036854775807.001", "-9223372036854775807",
}

func quantities() {
	r := rand.New(rand.NewSource(1))
	suffixes := []string{"", "n", "u", "m", "k", "M", "G", "T", "P", "E", "Ki", "Mi", "Gi", "Ti",
		"Pi", "Ei", "e3", "E3", "e-3", "e+2", "E-12", "e19", "e-30", "K", "mi", "KB", "i", "e", "x"}
	digits := func(n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			b.WriteByte(byte('0' + r.Intn(10)))
		}
		return b.String()
	}
	inputs := append([]string{}, fixedQuantities...)
	for i := 0; i < 3000; i++ {
		s := []string{"", "+", "-"}[r.Intn(3)] + digits(r.Intn(26))
		if r.Intn(2) == 0 {
			s += "." + digits(r.Intn(13))
		}
		s += suffixes[r.Intn(len(suffixes))]
		inputs = append(inputs, s)
	}
	for _, s := range inputs {
		q, err := resource.ParseQuantity(s)
		if err != nil {
			fmt.Printf("%s\terr\n", s)
			continue
		}
		fmt.Printf("%s\tok\t%s\t%d\n", s, q.AsDec().String(), q.Value())
	}
}

// Name inputs: the plan's tables and edge cases plus generated ones.
var fixedNames = []string{
	"", "My-App", "my_app", "my.app", "-app", "app-", "1web", "web-", "app..config", ".app",
	"app.", "app.-x", "app.my_x", "a/b/c", "/name", "example.com/", "Example.com/name", "_app",
	"app@1", "-web", "web_", "web app", "my-app", "a", "0", "web-1", "app.config.v1",
	"app.kubernetes.io/name", "App.Kubernetes", "example.com/Web_App-1.x", "Web", "v1.2_3",
	" app", "app ", "kube-system", "a.b/c", "UPPER", "x.Y/z",
}

func names() {
	r := rand.New(rand.NewSource(2))
	alphabet := "abcxyzABZ019-._/@ "
	valid := "abcxyz019"
	str := func(chars string, n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			b.WriteByte(chars[r.Intn(len(chars))])
		}
		return b.String()
	}
	kinds := []struct {
		name  string
		check func(string) []string
	}{
		{"dns_label", validation.IsDNS1123Label},
		{"dns1035_label", validation.IsDNS1035Label},
		{"dns_subdomain", validation.IsDNS1123Subdomain},
		{"label_key", validation.IsQualifiedName},
		{"label_value", validation.IsValidLabelValue},
	}
	for _, kind := range kinds {
		inputs := append([]string{}, fixedNames...)
		for _, n := range []int{62, 63, 64, 252, 253, 254} {
			inputs = append(inputs, str(valid, n), "x"+str(valid+".", n-2)+"x", "a/"+str(valid, n))
		}
		inputs = append(inputs, strings.Repeat("a", 64), strings.Repeat("a", 254))
		for i := 0; i < 400; i++ {
			inputs = append(inputs, str(alphabet, r.Intn(71)))
		}
		// Mostly-valid short names, so accepted inputs are well covered too.
		for i := 0; i < 600; i++ {
			var b strings.Builder
			for j := 0; j < 1+r.Intn(15); j++ {
				if r.Intn(5) == 0 {
					b.WriteString(string([]rune("-._/A@é")[r.Intn(7)]))
				} else {
					b.WriteByte(valid[r.Intn(len(valid))])
				}
			}
			inputs = append(inputs, b.String())
		}
		for _, s := range inputs {
			verdict := "ok"
			if len(kind.check(s)) > 0 {
				verdict = "err"
			}
			fmt.Printf("%s\t%s\t%s\n", kind.name, s, verdict)
		}
	}
}

// Selector inputs: the plan's table and examples plus mutated valid selectors.
var fixedSelectors = []string{
	"", " app=web", "app=web,", "app=web,,tier=db", ",app=web", "App@=web", "app=web app",
	"app=-web", "app in (a,)", "app in (,a)", "app in (a,,b)", "app in ()", "app notin ()",
	"app in (a", "app in a", "app > x", "!app=web", "app==", "=web", "app (a)",
	"app in (a, b)", "tier notin (x),env", "!canary", "replicas>3", "x<-5", "app=web AND",
	"a.b/c==d", "app=web", "app==web", "app!=web", "app", "!app", "app notin (a)",
	"app=,tier=db", "a.b/c=d, e in (f)", "app = web", "in=x", "notin", "app in (a b)",
	"app>3.5", "app<", "app=web=x", "!", "app!", "app!=", "app in (A_b-1.c)",
	"app=été", "!é", "a=(é", "a in (é)", "a=b,é", "é", "a!é", "a<é", "a>=é", "a==é",
}

func selectors() {
	r := rand.New(rand.NewSource(3))
	keys := []string{"app", "tier", "a.b/c", "example.com/name", "Env", "x", "k8s.io/ver", "ñ"}
	values := []string{"web", "db", "", "v1.2", "Prod", "-x", "1", "42", "a_b", "été"}
	requirement := func() []string {
		k := keys[r.Intn(len(keys))]
		v := values[r.Intn(len(values))]
		switch r.Intn(7) {
		case 0:
			return []string{k}
		case 1:
			return []string{"!", k}
		case 2:
			return []string{k, []string{"=", "==", "!="}[r.Intn(3)], v}
		case 3, 4:
			op := []string{"in", "notin"}[r.Intn(2)]
			set := []string{"("}
			for i := 0; i < 1+r.Intn(3); i++ {
				if i > 0 {
					set = append(set, ",")
				}
				set = append(set, values[r.Intn(len(values))])
			}
			return append([]string{k, op}, append(set, ")")...)
		default:
			return []string{k, []string{">", "<"}[r.Intn(2)], []string{"3", "-5", "x", "07"}[r.Intn(4)]}
		}
	}
	inserts := []string{",", "(", ")", "!", "=", "in", " "}
	inputs := append([]string{}, fixedSelectors...)
	for i := 0; i < 2000; i++ {
		var tokens []string
		for j := 0; j < 1+r.Intn(3); j++ {
			if j > 0 {
				tokens = append(tokens, ",")
			}
			tokens = append(tokens, requirement()...)
		}
		switch r.Intn(8) {
		case 0, 1, 2, 3: // valid, untouched
		case 4:
			k := r.Intn(len(tokens))
			tokens = append(tokens[:k], tokens[k+1:]...)
		case 5:
			k := r.Intn(len(tokens))
			tokens = append(tokens[:k+1], tokens[k:]...)
		case 6:
			a, b := r.Intn(len(tokens)), r.Intn(len(tokens))
			tokens[a], tokens[b] = tokens[b], tokens[a]
		default:
			k := r.Intn(len(tokens) + 1)
			tokens = append(tokens[:k], append([]string{inserts[r.Intn(len(inserts))]}, tokens[k:]...)...)
		}
		// Spaces between words; around symbols, sometimes none, as people write.
		var b strings.Builder
		for k, t := range tokens {
			if k > 0 {
				symbol := func(x string) bool { return x != "" && strings.ContainsAny(x[:1], "=!(),<>") }
				if (symbol(t) || symbol(tokens[k-1])) && r.Intn(2) == 0 {
					// no space
				} else {
					b.WriteString(" ")
				}
			}
			b.WriteString(t)
		}
		inputs = append(inputs, b.String())
	}
	for _, s := range inputs {
		verdict := "ok"
		if _, err := labels.Parse(s); err != nil {
			verdict = "err"
		}
		fmt.Printf("%s\t%s\n", s, verdict)
	}
}

// hexOf writes bytes as lower-case hex.
func hexOf(b []byte) string { return fmt.Sprintf("%x", b) }

// Digest inputs: the plan's table, known byte patterns and generated ones.
func fixedDigests() []string {
	h64 := strings.Repeat("ab", 32)
	h128 := strings.Repeat("cd", 64)
	seq := make([]byte, 64)
	for i := range seq {
		seq[i] = byte(i + 1)
	}
	return []string{
		"", " sha256:" + h64, h64, strings.Repeat("ef", 64), "abc", "sha384:" + strings.Repeat("a", 96),
		"md5:" + strings.Repeat("a", 32), "SHA256:" + h64, "sha256:" + h64[:63],
		"sha256:" + strings.ToUpper(h64), "sha256:" + h64[:63] + "g", "sha512:" + h128,
		"sha256:" + hexOf(make([]byte, 32)), "sha256:" + hexOf(seq[:32]), "sha512:" + hexOf(seq),
		"sha512:" + h128[:127], "sha256:", "sha256", ":" + h64,
	}
}

// randomDigest is shared with the image generator.
func randomDigest(r *rand.Rand) string {
	// Mostly the two supported algorithms, mostly clean, so accepted inputs are common.
	algs := []string{"sha256", "sha256", "sha256", "sha512", "sha512", "sha384", "SHA256", "md5", ""}
	lengths := map[string]int{"sha256": 64, "sha512": 128, "sha384": 96, "SHA256": 64, "md5": 32, "": 64}
	alg := algs[r.Intn(len(algs))]
	n := r.Intn(131)
	if r.Intn(3) != 0 {
		n = lengths[alg]
	}
	dirty := r.Intn(3) == 0
	var b strings.Builder
	for i := 0; i < n; i++ {
		pick := 2 + r.Intn(38)
		if dirty {
			pick = r.Intn(20)
		}
		switch pick {
		case 0:
			b.WriteByte("ABCDEF"[r.Intn(6)])
		case 1:
			b.WriteString([]string{"g", "é"}[r.Intn(2)])
		default:
			b.WriteByte("0123456789abcdef"[r.Intn(16)])
		}
	}
	sep := ":"
	if r.Intn(10) == 0 {
		sep = ""
	}
	return alg + sep + b.String()
}

func digests() {
	r := rand.New(rand.NewSource(4))
	inputs := fixedDigests()
	for i := 0; i < 1500; i++ {
		inputs = append(inputs, randomDigest(r))
	}
	for _, s := range inputs {
		verdict := "ok"
		if _, err := digest.Parse(s); err != nil {
			verdict = "err"
		}
		fmt.Printf("%s\t%s\n", s, verdict)
	}
}

// Image reference inputs: the plan's table and focus cases plus generated ones.
func images() {
	d := "sha256:" + strings.Repeat("ab", 32)
	hex64 := strings.Repeat("0123456789abcdef", 4)
	fixed := []string{
		"", " nginx", "nginx :1", "https://ghcr.io/org/app", "ghcr.io/Org/App:1.2", "MyOrg/app",
		"org/app-", "org//app", "org/_app", "a..b", "exa_mple.com/app", "host:x/app", "nginx:",
		"nginx:-x", "nginx:a+b", "nginx:" + strings.Repeat("t", 129), "nginx@", "nginx@latest",
		"nginx@sha256:" + strings.Repeat("ab", 31) + "a", hex64, strings.Repeat("a", 256),
		"x/" + strings.Repeat("a", 254), "nginx", "nginx:1.27", "user/app", "docker.io/nginx",
		"index.docker.io/nginx", "localhost/app", "localhost:5000/app",
		"registry.example.com:5000/app", "app:5000", "registry:5000/app:1.2",
		"Registry.Example.com/app", "app:V1", "nginx@" + d, "ghcr.io/org/app:1.2@" + d,
		"[::1]:5000/app", "127.0.0.1:5000/a/b:c", "a__b/c---d", "nginx:latest",
		"nginx@sha384:" + strings.Repeat("a", 96), "localhost", "docker.io/library/nginx:latest",
		"nginx:" + strings.Repeat("t", 128), "Org/app", "org/App",
	}
	r := rand.New(rand.NewSource(5))
	registries := []string{"", "", "", "ghcr.io/", "localhost/", "localhost:5000/", "Reg.io/", "MyOrg/",
		"index.docker.io/", "docker.io/", "[::1]:5000/", "a_b.io/"}
	component := func() string {
		var b strings.Builder
		for i := 0; i < 1+r.Intn(8); i++ {
			switch r.Intn(25) {
			case 0:
				b.WriteString([]string{".", "_", "-", "__", "A", "é"}[r.Intn(6)])
			default:
				b.WriteByte("abcxyz019"[r.Intn(9)])
			}
		}
		return b.String()
	}
	// The upstream package's own test literals (upstream.go), as the spec asks.
	inputs := append(fixed, upstreamReferences...)
	for i := 0; i < 2000; i++ {
		s := registries[r.Intn(len(registries))]
		for j := 0; j < 1+r.Intn(3); j++ {
			if j > 0 {
				s += "/"
			}
			s += component()
		}
		if r.Intn(2) == 0 {
			var t strings.Builder
			for k := 0; k < 1+r.Intn(10); k++ {
				t.WriteByte("abcXYZ019_.-"[r.Intn(12)])
			}
			s += ":" + t.String()
		}
		if r.Intn(4) == 0 {
			s += "@" + randomDigest(r)
		}
		inputs = append(inputs, s)
	}
	dash := func(s string) string {
		if s == "" {
			return "-"
		}
		return s
	}
	for _, s := range inputs {
		named, err := reference.ParseNormalizedNamed(s)
		if err != nil {
			fmt.Printf("%s\terr\n", s)
			continue
		}
		tag, digestText := "", ""
		if t, ok := named.(reference.Tagged); ok {
			tag = t.Tag()
		}
		if dg, ok := named.(reference.Digested); ok {
			digestText = dg.Digest().String()
		}
		full := reference.TagNameOnly(named)
		fmt.Printf("%s\tok\t%s\t%s\t%s\t%s\t%s\n", s, full.String(), reference.Domain(named),
			reference.Path(named), dash(tag), dash(digestText))
	}
}

// Platform inputs: Go's own os/arch pairs, the plan's table and aliases, and
// generated ones. containerd's Normalize is the oracle for alias suggestions.
func platformsFixture() {
	out, err := exec.Command("go", "tool", "dist", "list").Output()
	if err != nil {
		fmt.Fprintln(os.Stderr, "go tool dist list:", err)
		os.Exit(1)
	}
	dist := strings.Fields(string(out))
	for _, pair := range dist {
		fmt.Printf("dist\t%s\n", pair)
	}
	fixed := []string{
		"", " linux/amd64", "linux", "linux/arm/v7/x", "linux//amd64", "linux/amd64/", "Linux/AMD64",
		"macos/arm64", "osx/arm64", "linux/x86_64", "linux/x86-64", "linux/aarch64", "linux/armv7",
		"linux/armhf", "linux/armv6", "linux/armel", "linux/i386", "linux/i686", "linux/x86",
		"lnux/amd64", "linux/amd46", "linux/amd64/v8", "linux/386/v1", "linux/arm/7",
		"linux/arm64/v8", "linux/arm/v7", "windows/amd64", "darwin/arm64", "linux/arm64/v9.5",
		"linux/amd64/v3", "wasip1/wasm", "linux/s390x", "linux/arm64", "linux/arm/v5",
		"linux/arm64/v8.1", "linux/riscv64",
	}
	r := rand.New(rand.NewSource(6))
	oses := []string{"linux", "windows", "darwin", "freebsd", "macos", "Linux", "wasip1", "lnux"}
	arches := []string{"amd64", "arm64", "arm", "386", "riscv64", "x86_64", "aarch64", "armhf", "i386", "s390x", "ppc64le", "AMD64"}
	variants := []string{"", "", "", "v1", "v3", "v5", "v6", "v7", "v8", "v8.2", "v9", "7"}
	inputs := append(fixed, dist...)
	for i := 0; i < 1000; i++ {
		s := oses[r.Intn(len(oses))] + "/" + arches[r.Intn(len(arches))]
		if v := variants[r.Intn(len(variants))]; v != "" {
			s += "/" + v
		}
		inputs = append(inputs, s)
	}
	for _, s := range inputs {
		p, err := platforms.Parse(s)
		if err != nil {
			fmt.Printf("parse\t%s\terr\n", s)
			continue
		}
		fmt.Printf("parse\t%s\tok\t%s\n", s, platforms.Format(platforms.Normalize(p)))
	}
}
